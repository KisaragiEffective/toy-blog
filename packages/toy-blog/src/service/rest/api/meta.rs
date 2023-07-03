use actix_web::Responder;
use actix_web::web::Query;
use actix_web::post;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use toy_blog_endpoint_model::{ChangeArticleIdError, ChangeArticleIdRequestQuery, ChangeArticleIdRequestResult};
use crate::GLOBAL_FILE;
use crate::service::rest::auth::{is_wrong_token};
use crate::service::rest::exposed_representation_format::EndpointRepresentationCompiler;
use crate::service::rest::ComposeInternalError;
use crate::service::rest::inner_no_leak::UnhandledError;
use crate::service::persistence::PersistenceError;

#[post("/change-id")]
pub async fn change_id(query: Query<ChangeArticleIdRequestQuery>, bearer: BearerAuth) -> impl Responder {
    let token = bearer.token();

    let ChangeArticleIdRequestQuery { from, to } = query.into_inner();

    let res: ComposeInternalError<ChangeArticleIdRequestResult> = (|| {
        if is_wrong_token(token) {
            return Ok(Err(ChangeArticleIdError::Unauthorized))
        }

        match GLOBAL_FILE.get().expect("must be fully-initialized").rename(&from, to) {
            Ok(_) => {
                Ok(Ok(()))
            }
            Err(e) => {
                match e {
                    PersistenceError::AbsentValue => {
                        Ok(Err(ChangeArticleIdError::ArticleNotFoundById))
                    }
                    other => Err(UnhandledError::new(other)),
                }
            }
        }
    })();

    EndpointRepresentationCompiler::from_value(res).into_plain_text()
}
