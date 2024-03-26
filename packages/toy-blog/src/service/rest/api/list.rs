use actix_web::{get, Responder};
use actix_web::web::Path;
use chrono::Datelike;

use toy_blog_endpoint_model::{AnnoDominiYear, Article, ArticleIdSet, ArticleIdSetMetadata, OneOriginTwoDigitsMonth, OwnedMetadata, Visibility};

use crate::service::rest::exposed_representation_format::{ArticleIdCollectionResponseRepr, EndpointRepresentationCompiler};
use crate::service::rest::repository::GLOBAL_FILE;

#[get("/article")]
#[allow(clippy::unused_async)]
pub async fn article_id_list() -> impl Responder {
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let x = GLOBAL_FILE.get().expect("must be fully-initialized").entries();

    let id = ArticleIdSet(x.iter().filter(|x| x.1.visibility == Visibility::Public).map(|x| &x.0).cloned().collect());
    let old_cre = x.iter().min_by_key(|x| x.1.created_at).map(|x| x.1.created_at);
    let new_upd = x.iter().max_by_key(|x| x.1.updated_at).map(|x| x.1.updated_at);

    EndpointRepresentationCompiler::from_value(
        ArticleIdCollectionResponseRepr(OwnedMetadata {
            metadata: ArticleIdSetMetadata {
                oldest_created_at: old_cre,
                newest_updated_at: new_upd,
            },
            data: id
        })
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}

#[get("/article/{year}")]
#[allow(clippy::unused_async)]
pub async fn article_id_list_by_year(path: Path<AnnoDominiYear>) -> impl Responder {
    let year = path.into_inner();
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let f = |a: &Article| a.created_at.year() == year.into_inner() as i32 && a.visibility == Visibility::Public;
    let x = GLOBAL_FILE.get().expect("must be fully-initialized").entries();

    let (matched_article_id_set, oldest_creation, most_recent_update) = (
        ArticleIdSet(
            x.iter().filter(|x| f(&x.1)).map(|x| &x.0).cloned().collect()
        ),
        x.iter().filter(|x| f(&x.1)).min_by_key(|x| x.1.created_at).map(|x| x.1.created_at),
        x.iter().filter(|x| f(&x.1)).max_by_key(|x| x.1.updated_at).map(|x| x.1.updated_at),
    );

    EndpointRepresentationCompiler::from_value(
        ArticleIdCollectionResponseRepr(OwnedMetadata {
            metadata: ArticleIdSetMetadata {
                oldest_created_at: oldest_creation,
                newest_updated_at: most_recent_update,
            },
            data: matched_article_id_set
        })
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}

#[get("/article/{year}/{month}")]
#[allow(clippy::unused_async)]
pub async fn article_id_list_by_year_and_month(
    path: Path<(AnnoDominiYear, OneOriginTwoDigitsMonth)>
) -> impl Responder {
    let (year, month) = path.into_inner();
    let f = |a: &Article| {
        #[allow(clippy::cast_possible_wrap)] // effectively FP, AnnoDominiYear is <= 2147483647; are we going to use this product even if after that? ;)
            let filter_year = a.created_at.year() == year.into_inner() as i32;
        // SAFETY: `month()` returns 1..=12, which is subset of possible u8 value.
        let article_created_month = unsafe {
            u8::try_from(a.created_at.month()).unwrap_unchecked()
        };
        let filter_month = article_created_month == month.into_inner();

        filter_year && filter_month && a.visibility == Visibility::Public
    };
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let x = GLOBAL_FILE.get().expect("must be fully-initialized").entries();

    let (matched_article_id_set, oldest_creation, most_recent_update) = (
        ArticleIdSet(
            x.iter().filter(|x| f(&x.1)).map(|x| &x.0).cloned().collect()
        ),
        x.iter().filter(|x| f(&x.1)).min_by_key(|x| x.1.created_at).map(|x| x.1.created_at),
        x.iter().filter(|x| f(&x.1)).max_by_key(|x| x.1.updated_at).map(|x| x.1.updated_at),
    );

    EndpointRepresentationCompiler::from_value(
        ArticleIdCollectionResponseRepr(OwnedMetadata {
            metadata: ArticleIdSetMetadata {
                oldest_created_at: oldest_creation,
                newest_updated_at: most_recent_update,
            },
            data: matched_article_id_set
        })
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body()
}
