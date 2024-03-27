use actix_web::{get, Responder};
use actix_web::web::Path;
use chrono::Datelike;

use toy_blog_endpoint_model::{AnnoDominiYear, Article, ArticleId, ArticleIdSet, ArticleIdSetMetadata, OneOriginTwoDigitsMonth, OwnedMetadata, Visibility};

use crate::service::rest::exposed_representation_format::{ArticleIdCollectionResponseRepr, EndpointRepresentationCompiler, MaybeNotModified, ReportLastModofied};
use crate::service::rest::header::IfModifiedSince;
use crate::service::rest::repository::GLOBAL_FILE;

fn compute_and_filter_out(
    if_modified_since: Option<IfModifiedSince>,
    additional_filter: impl Clone + Fn(&&(ArticleId, Article)) -> bool
) -> ArticleIdCollectionResponseRepr {
    let x = GLOBAL_FILE.get().expect("must be fully-initialized").entries();
    let only_public = |x: &&(ArticleId, Article)| x.1.visibility == Visibility::Public;
    let ret_304;
    let latest_updated = x.iter()
        .filter(only_public)
        .filter(additional_filter.clone())
        .max_by_key(|r| r.1.updated_at)
        .map(|x| &x.1).map(|x| x.updated_at);

    if let Some(if_modified_since) = if_modified_since {
        let if_unmodified_since = if_modified_since.0.0;
        ret_304 = latest_updated.map(|d| d >= if_unmodified_since).unwrap_or(false);
    } else {
        ret_304 = false;
    }

    let id = ArticleIdSet(x.iter().filter(only_public).filter(additional_filter.clone())
        .map(|x| &x.0).cloned().collect());
    let old_cre = x.iter().filter(only_public).filter(additional_filter.clone())
        .min_by_key(|x| x.1.created_at).map(|x| x.1.created_at);
    let new_upd = x.iter().filter(only_public).filter(additional_filter)
        .max_by_key(|x| x.1.updated_at).map(|x| x.1.updated_at);

    ArticleIdCollectionResponseRepr(
        MaybeNotModified {
            inner: ReportLastModofied {
                inner: OwnedMetadata {
                    metadata: ArticleIdSetMetadata {
                        oldest_created_at: old_cre,
                        newest_updated_at: new_upd,
                    },
                    data: id
                },
                latest_updated: latest_updated.map(|x| x.into())
            },
            is_modified: ret_304,
        }
    )
}

#[get("/article")]
#[allow(clippy::unused_async)]
pub async fn article_id_list(if_modified_since: Option<IfModifiedSince>) -> impl Responder {
    EndpointRepresentationCompiler::from_value(
        compute_and_filter_out(if_modified_since, |_| true)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}

#[get("/article/{year}")]
#[allow(clippy::unused_async)]
pub async fn article_id_list_by_year(path: Path<AnnoDominiYear>, if_modified_since: Option<IfModifiedSince>) -> impl Responder {
    let year = path.into_inner().into_inner();
    EndpointRepresentationCompiler::from_value(
        compute_and_filter_out(if_modified_since, |x| x.1.created_at.year() as u32 == year)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}

#[get("/article/{year}/{month}")]
#[allow(clippy::unused_async)]
pub async fn article_id_list_by_year_and_month(
    path: Path<(AnnoDominiYear, OneOriginTwoDigitsMonth)>, if_modified_since: Option<IfModifiedSince>
) -> impl Responder {
    let (year, month) = path.into_inner();
    let year = year.into_inner();
    let month = month.into_inner();

    EndpointRepresentationCompiler::from_value(
        compute_and_filter_out(if_modified_since, |x| x.1.created_at.year() as u32 == year && x.1.created_at.month() as u8 == month)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}
