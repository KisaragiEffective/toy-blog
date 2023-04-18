use std::cmp::Reverse;
use actix_web::{HttpRequest, Responder};
use actix_web::{delete, get, post, put};
use actix_web::body::BoxBody;
use actix_web::http::header::USER_AGENT;
use actix_web::web::{Bytes, Path, Query};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use log::info;
use once_cell::unsync::Lazy;
use toy_blog_endpoint_model::{
    ArticleContent,
    ArticleCreatedNotice,
    ArticleCreateWarning,
    ArticleId,
    ArticleSnapshot,
    ArticleSnapshotMetadata,
    CreateArticleError,
    DeleteArticleError,
    GetArticleError,
    ListArticleRequestQuery,
    ListArticleResult,
    OwnedMetadata,
    UpdateArticleError,
    ListArticleResponse
};
use crate::service::http::repository::GLOBAL_FILE;
use crate::service::http::auth::is_wrong_token;
use crate::service::http::inner_no_leak::{ComposeInternalError, UnhandledError};
use super::super::exposed_representation_format::EndpointRepresentationCompiler;

#[post("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn create(path: Path<String>, data: Bytes, bearer: BearerAuth, request: HttpRequest) -> impl Responder {
    let token = bearer.token();
    let res = || async {
        if is_wrong_token(token) {
            return Ok(Err(CreateArticleError::Unauthorized))
        }

        let path = ArticleId::new(path.into_inner());
        info!("create");
        if GLOBAL_FILE.exists(&path).await.unwrap() {
            return Ok(Err(CreateArticleError::DuplicatedArticleId))
        }

        let plain_text = String::from_utf8(data.to_vec());
        let text = match plain_text {
            Ok(s) => s,
            Err(_) => return Ok(Err(CreateArticleError::InvalidUtf8))
        };

        info!("valid utf8");
        let res = GLOBAL_FILE.create_entry(&path, text.clone()).await;
        match res {
            Ok(_) => {}
            Err(err) => return Err(UnhandledError::new(err))
        }

        let curl_like = request.headers().get(USER_AGENT)
            .and_then(|ua| ua.to_str().ok())
            .map_or(false, |ua| ua.starts_with("curl/"));

        let no_newline = Lazy::new(|| text.contains('\n'));

        let warnings = if curl_like && *no_newline {
            maplit::hashset![
                ArticleCreateWarning::CurlSpecificNoNewLine
            ]
        } else {
            maplit::hashset![]
        };

        Ok(Ok(ArticleCreatedNotice {
            warnings,
            allocated_id: path,
        }))
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}

#[get("/{article_id}")]
pub async fn fetch(path: Path<String>) -> impl Responder {
    let res = || async {
        let article_id = ArticleId::new(path.into_inner());

        let exists = match GLOBAL_FILE.exists(&article_id).await {
            Ok(exists) => exists,
            Err(e) => return Err(UnhandledError::new(e))
        };

        if !exists {
            return Ok(Err(GetArticleError::NoSuchArticleFoundById))
        }

        let content = match GLOBAL_FILE.read_snapshot(&article_id).await {
            Ok(content) => content,
            Err(e) => return Err(UnhandledError::new(e))
        };

        let u = content.updated_at;
        let uo = u.offset();
        let uu = u.with_timezone(uo);

        Ok(Ok(OwnedMetadata {
            metadata: ArticleSnapshotMetadata {
                updated_at: uu
            },
            data: ArticleSnapshot {
                content: ArticleContent::new(content.content)
            },
        }))
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}

#[put("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn update(path: Path<String>, data: Bytes, bearer: BearerAuth) -> impl Responder {
    let res = || async {
        let token = bearer.token();

        if is_wrong_token(token) {
            return Ok(Err(UpdateArticleError::InvalidBearerToken))
        }

        let article_id = ArticleId::new(path.into_inner());

        let exists = match GLOBAL_FILE.exists(&article_id).await {
            Ok(exists) => exists,
            Err(e) => return Err(UnhandledError::new(e))
        };

        if !exists {
            return Ok(Err(UpdateArticleError::ArticleNotFoundById))
        }

        let data = match String::from_utf8(data.to_vec()) {
            Ok(data) => data,
            Err(e) => return Ok(Err(UpdateArticleError::InvalidByteSequenceForUtf8(e)))
        };

        match GLOBAL_FILE.update_entry(&article_id, data).await {
            Ok(_) => {
                Ok(Ok(()))
            }
            Err(err) => {
                Err(UnhandledError::new(err))
            }
        }
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}

#[delete("/{article_id}")]
#[allow(clippy::future_not_send)]
pub async fn remove(path: Path<String>, bearer: BearerAuth) -> impl Responder {
    let res = || async {
        let article_id = ArticleId::new(path.into_inner());
        let token = bearer.token();

        if is_wrong_token(token) {
            return Ok(Err(DeleteArticleError::InvalidBearerToken))
        }

        let exists = match GLOBAL_FILE.exists(&article_id).await {
            Ok(exists) => exists,
            Err(err) => return Err(UnhandledError::new(err))
        };

        if !exists {
            return Ok(Err(DeleteArticleError::NoSuchArticleFoundById))
        }

        match GLOBAL_FILE.remove(&article_id).await {
            Ok(_) => {
                Ok(Ok(()))
            }
            Err(err) => {
                Err(UnhandledError::new(err))
            }
        }
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}
