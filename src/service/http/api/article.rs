use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::{delete, get, post, put};
use actix_web::http::header::{LAST_MODIFIED, USER_AGENT};
use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Path};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::{DateTime, FixedOffset, TimeZone};
use log::info;
use crate::service::persistence::ListOperationScheme;
use crate::service::persistence::model::ArticleId;
use crate::service::http::repository::GLOBAL_FILE;
use crate::extension::RespondPlainText;
use crate::GIVEN_TOKEN;

#[post("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn create(path: Path<String>, data: Bytes, bearer: BearerAuth, request: HttpRequest) -> impl Responder {
    let token = bearer.token();
    if is_wrong_token(token) {
        return unauthorized()
    }

    let path = ArticleId::new(path.into_inner());
    info!("create");
    if GLOBAL_FILE.exists(&path).await.unwrap() {
        return HttpResponse::build(StatusCode::CONFLICT)
            .respond_with_auto_charset("already exist. Please choose another one, or overwrite with PUT request.")
    }

    let plain_text = String::from_utf8(data.to_vec());
    if let Ok(text) = plain_text {
        info!("valid utf8");
        let res = GLOBAL_FILE.create_entry(&path, text.clone()).await;
        match res {
            Ok(_) => {
                let warnings = if let Some(user_agent) = request.headers().get(USER_AGENT) {
                    if user_agent.to_str().unwrap().starts_with("curl/") && !text.contains('\n') {
                        vec![
                            r#"There's no newlines. Perhaps you should use --data-binary instead?
Note: `man curl(1)` said:
    > When -d, --data is told to read from a file like that, carriage
    > returns and newlines  will  be stripped out."#
                        ]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                let warnings = warnings
                    .into_iter()
                    .map(|a| a.to_string() + "\n")
                    .collect::<Vec<_>>()
                    .join("");

                HttpResponse::build(StatusCode::OK)
                    .respond_with_auto_charset(format!("{warnings}OK, saved as {path}.", path = &path))
            }
            Err(err) => {
                HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                    .respond_with_auto_charset(format!("Exception: {err}"))
            }
        }
    } else {
        info!("invalid utf8");
        HttpResponse::build(StatusCode::BAD_REQUEST)
            .body("text must be valid UTF-8")
    }
}

fn fmt_http_date<Tz: TimeZone>(dt: &DateTime<Tz>) -> String {
    let gmt_datetime = dt.with_timezone(&FixedOffset::east(0));
    // Last-Modified: <day-name>, <day> <month> <year> <hour>:<minute>:<second> GMT
    gmt_datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

#[get("/{article_id}")]
pub async fn fetch(path: Path<String>) -> impl Responder {
    let article_id = ArticleId::new(path.into_inner());
    match GLOBAL_FILE.exists(&article_id).await {
        Ok(exists) => {
            if exists {
                let content = GLOBAL_FILE.read_snapshot(&article_id).await;
                match content {
                    Ok(content) => {
                        HttpResponse::build(StatusCode::OK)
                            // compliant with RFC 7232 (HTTP/1.1 Conditional Requests) § 2.1.1
                            .insert_header((LAST_MODIFIED, fmt_http_date(&content.updated_at)))
                            // TODO: Having ETag is fun, right?
                            .respond_with_auto_charset(content.content)
                    }
                    Err(err) => {
                        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                            .respond_with_auto_charset(format!("Exception {err}"))
                    }
                }
            } else {
                HttpResponse::build(StatusCode::NOT_FOUND)
                    .respond_with_auto_charset("Not found")
            }
        }
        Err(err) => {
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .respond_with_auto_charset(format!("Exception {err}"))
        }
    }
}

#[put("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn update(path: Path<String>, data: Bytes, bearer: BearerAuth) -> impl Responder {
    let token = bearer.token();

    if is_wrong_token(token) {
        return unauthorized()
    }

    let article_id = ArticleId::new(path.into_inner());
    match GLOBAL_FILE.exists(&article_id).await {
        Ok(exists) => {
            if exists {
                let data = match String::from_utf8(data.to_vec()) {
                    Ok(s) => s,
                    Err(e) => {
                        return HttpResponse::build(StatusCode::BAD_REQUEST)
                            .respond_with_auto_charset(format!("You must provide valid UTF-8 sequence: {e}"))
                    }
                };

                match GLOBAL_FILE.update_entry(&article_id, data).await {
                    Ok(_) => {
                        HttpResponse::build(StatusCode::NO_CONTENT)
                            .respond_with_auto_charset("saved")
                    }
                    Err(err) => {
                        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                            .respond_with_auto_charset(format!("Exception {err}"))
                    }
                }
            } else {
                HttpResponse::build(StatusCode::NOT_FOUND)
                    .respond_with_auto_charset("Not found")
            }
        }
        Err(err) => {
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .respond_with_auto_charset(format!("Exception {err}"))
        }
    }
}

#[delete("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn remove(path: Path<String>, bearer: BearerAuth) -> impl Responder {
    let article_id = ArticleId::new(path.into_inner());
    let token = bearer.token();
    if is_wrong_token(token) {
        return unauthorized()
    }

    match GLOBAL_FILE.exists(&article_id).await {
        Ok(exists) => {
            if exists {
                match GLOBAL_FILE.remove(&article_id).await {
                    Ok(_) => {
                        HttpResponse::build(StatusCode::NO_CONTENT)
                            .respond_with_auto_charset("deleted")
                    }
                    Err(err) => {
                        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                            .respond_with_auto_charset(format!("Exception {err}"))
                    }
                }
            } else {
                HttpResponse::build(StatusCode::NOT_FOUND)
                    .respond_with_auto_charset("Not found")
            }
        }
        Err(err) => {
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .respond_with_auto_charset(format!("Exception {err}"))
        }
    }
}

#[get("/articles")]
#[allow(clippy::unused_async)]
pub async fn list() -> impl Responder {
    match GLOBAL_FILE.parse_file_as_json() {
        Ok(entries) => {
            HttpResponse::build(StatusCode::OK)
                .json(ListOperationScheme::from(entries))
        }
        Err(err) => {
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .respond_with_auto_charset(format!("Exception {err}"))
        }
    }
}

fn is_wrong_token(token: &str) -> bool {
    let correct_token = GIVEN_TOKEN.get().unwrap().as_str();
    correct_token != token
}

fn unauthorized() -> HttpResponse {
    HttpResponse::build(StatusCode::UNAUTHORIZED)
        .respond_with_auto_charset("You must be authorized to perform this action.")
}
