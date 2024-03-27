
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::{delete, get, post, put};

use actix_web::http::header::USER_AGENT;
use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Json, Path};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use log::{error, info};
use once_cell::unsync::Lazy;
use toy_blog_endpoint_model::{ArticleContent, ArticleCreatedNotice, ArticleCreateWarning, ArticleId, ArticleSnapshot, ArticleSnapshotMetadata, CreateArticleError, DeleteArticleError, GetArticleError, OwnedMetadata, UpdateArticleError, UpdateVisibilityPayload, Visibility};
use crate::service::rest::auth::is_wrong_token;
use crate::service::rest::inner_no_leak::{UnhandledError};
use crate::service::rest::repository::GLOBAL_FILE;
use crate::service::persistence::ArticleRepository;
use super::super::exposed_representation_format::EndpointRepresentationCompiler;

fn x_get<'a>() -> &'a ArticleRepository {
    GLOBAL_FILE.get().expect("must be fully-initialized")
}

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
        if x_get().exists(&path) {
            return Ok(Err(CreateArticleError::DuplicatedArticleId))
        }

        let plain_text = String::from_utf8(data.to_vec());
        let Ok(text) = plain_text else { return Ok(Err(CreateArticleError::InvalidUtf8)) };

        info!("valid utf8");
        let res = x_get().create_entry(&path, text.clone(), Visibility::Private);
        match res {
            Ok(()) => {}
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
pub async fn fetch(path: Path<String>, auth: Option<BearerAuth>) -> impl Responder {

    enum Res {
        Internal(UnhandledError),
        General(GetArticleError),
        Ok(OwnedMetadata<ArticleSnapshotMetadata, ArticleSnapshot>),
    }

    let res = || async {
        let article_id = ArticleId::new(path.into_inner());

        let exists = x_get().exists(&article_id);

        if !exists {
            return Res::General(GetArticleError::NoSuchArticleFoundById)
        }

        let content = match x_get().read_snapshot(&article_id) {
            Ok(content) => content,
            Err(e) => return Res::Internal(UnhandledError::new(e))
        };

        // Visibility::Restricted, Visibility::Publicは検証不要
        if let (Visibility::Private, Some(auth)) = (content.visibility, auth) {
            if is_wrong_token(auth.token()) {
                return Res::General(GetArticleError::NoSuchArticleFoundById)
            }
            // now, private article can see from permitted user!
        }

        let u = content.updated_at;
        let uo = u.offset();
        let uu = u.with_timezone(uo);

        Res::Ok(OwnedMetadata {
            metadata: ArticleSnapshotMetadata {
                updated_at: uu
            },
            data: ArticleSnapshot {
                content: ArticleContent::new(content.content)
            },
        })
    };

    let x = match res().await {
        Res::Internal(sre) => {
            error!("{sre:?}");
            return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Res::General(e) => Err(e),
        Res::Ok(v) => Ok(v)
    };

    EndpointRepresentationCompiler::from_value(x).into_plain_text().map_into_boxed_body()
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

        let exists = x_get().exists(&article_id);

        if !exists {
            return Ok(Err(UpdateArticleError::ArticleNotFoundById))
        }

        let data = match String::from_utf8(data.to_vec()) {
            Ok(data) => data,
            Err(e) => return Ok(Err(UpdateArticleError::InvalidByteSequenceForUtf8(e)))
        };

        match x_get().update_entry(&article_id, data) {
            Ok(()) => {
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

        let exists = x_get().exists(&article_id);

        if !exists {
            return Ok(Err(DeleteArticleError::NoSuchArticleFoundById))
        }

        match x_get().remove(&article_id) {
            Ok(()) => {
                Ok(Ok(()))
            }
            Err(err) => {
                Err(UnhandledError::new(err))
            }
        }
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}

#[put("/{article_id}/visibility")]
pub async fn update_visibility(path: Path<String>, payload: Json<UpdateVisibilityPayload>, bearer: BearerAuth) -> impl Responder {
    let res = || async {
        let article_id = ArticleId::new(path.into_inner());
        let token = bearer.token();

        if is_wrong_token(token) {
            return Ok(Err(DeleteArticleError::InvalidBearerToken))
        }

        let exists = x_get().exists(&article_id);

        if !exists {
            return Ok(Err(DeleteArticleError::NoSuchArticleFoundById))
        }

        let new_visibility = payload.visibility;
        match x_get().change_visibility(&article_id, new_visibility) {
            Ok(()) => {
                Ok(Ok(()))
            }
            Err(err) => {
                Err(UnhandledError::new(err))
            }
        }
    };

    EndpointRepresentationCompiler::from_value(res().await).into_plain_text()
}
