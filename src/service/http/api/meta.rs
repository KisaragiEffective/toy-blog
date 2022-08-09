use actix_web::{HttpResponse, Responder};
use actix_web::http::StatusCode;
use actix_web::web::Query;
use actix_web::post;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::Deserialize;
use crate::extension::RespondPlainText;
use crate::{ArticleId, GLOBAL_FILE};
use crate::service::http::auth::{is_wrong_token, unauthorized};

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyQuery {
    from: ArticleId,
    to: ArticleId,
}

#[post("/change-id")]
pub async fn change_id(query: Query<KeyQuery>, bearer: BearerAuth) -> impl Responder {
    let token = bearer.token();
    if is_wrong_token(token) {
        return unauthorized()
    }

    let KeyQuery { from, to } = query.into_inner();
    match GLOBAL_FILE.rename(&from, to) {
        Ok(_) => {
            HttpResponse::build(StatusCode::OK)
                .respond_with_auto_charset("The article was successfully renamed")
        }
        Err(_) => {
            HttpResponse::build(StatusCode::BAD_REQUEST)
                .respond_with_auto_charset("The article does not exist")
        }
    }
}

