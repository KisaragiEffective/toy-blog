use std::collections::HashSet;
use actix_web::{get, Responder};
use actix_web::web::Path;
use chrono::Datelike;
use crate::service::http::exposed_representation_format::{ArticleIdCollectionResponseRepr, EndpointRepresentationCompiler, InternalErrorExposedRepr};
use crate::service::http::repository::GLOBAL_FILE;
use toy_blog_endpoint_model::{
    ArticleId, ArticleIdSet,
    OwnedMetadata,
    ArticleIdSetMetadata,
    AnnoDominiYear,
    OneOriginTwoDigitsMonth,
    Article,
};

#[get("/article")]
pub async fn article_id_list() -> impl Responder {
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let x = GLOBAL_FILE.parse_file_as_json()
        .map(|x|
            (
                ArticleIdSet(x.data.keys().cloned().collect::<HashSet<_>>()),
                // oldest creation
                x.data.values().min_by_key(|x| x.created_at).map(|x| x.created_at),
                // newest update
                x.data.values().max_by_key(|x| x.updated_at).map(|x| x.updated_at),
            )
        );

    match x {
        Ok((l, old_cre, new_upd)) =>
            EndpointRepresentationCompiler::from_value(
                ArticleIdCollectionResponseRepr(OwnedMetadata {
                    metadata: ArticleIdSetMetadata {
                        oldest_created_at: old_cre,
                        newest_updated_at: new_upd,
                    },
                    data: l
                })
            ).into_json()
                .map_body(|_, y| serde_json::to_string(&y).expect(""))
                .map_into_boxed_body(),
        Err(e) => EndpointRepresentationCompiler::from_value(
            InternalErrorExposedRepr(Box::new(e))
        ).into_json()
            .map_body(|_, y| serde_json::to_string(&y).expect(""))
            .map_into_boxed_body()
    }
}

#[get("/article/{year}")]
pub async fn article_id_list_by_year(path: Path<AnnoDominiYear>) -> impl Responder {
    let year = path.into_inner();
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let f = |(_, a): &(&ArticleId, &Article)| a.created_at.year() == year.into_inner() as i32;
    let x = GLOBAL_FILE.parse_file_as_json()
        .map(|x|
            (
                ArticleIdSet(
                    x.data.iter()
                        .filter(f)
                        .map(|(x, _)| x)
                        .cloned()
                        .collect::<HashSet<_>>()
                ),
                // oldest creation
                x.data.iter()
                    .filter(f)
                    .min_by_key(|(_, x)| x.created_at)
                    .map(|(_, x)| x.created_at),
                // newest update
                x.data.iter()
                    .filter(f)
                    .max_by_key(|(_, x)| x.updated_at)
                    .map(|(_, x)| x.updated_at),
            )
        );

    match x {
        Ok((l, old_cre, new_upd)) =>
            EndpointRepresentationCompiler::from_value(
                ArticleIdCollectionResponseRepr(OwnedMetadata {
                    metadata: ArticleIdSetMetadata {
                        oldest_created_at: old_cre,
                        newest_updated_at: new_upd,
                    },
                    data: l
                })
            ).into_json()
                .map_body(|_, y| serde_json::to_string(&y).expect(""))
                .map_into_boxed_body(),
        Err(e) => EndpointRepresentationCompiler::from_value(
            InternalErrorExposedRepr(Box::new(e))
        ).into_json()
            .map_body(|_, y| serde_json::to_string(&y).expect(""))
            .map_into_boxed_body()
    }
}

#[get("/article/{year}/{month}")]
pub async fn article_id_list_by_year_and_month(
    path: Path<(AnnoDominiYear, OneOriginTwoDigitsMonth)>
) -> impl Responder {
    let (year, month) = path.into_inner();
    let f = |(_, a): &(&ArticleId, &Article)| {
        (a.created_at.year() == year.into_inner() as i32) && (a.created_at.month() as u8 == month.into_inner())
    };
    // TODO: DBに切り替えたら、ヘッダーを受け取るようにし、DB上における最大値と最小値を確認して条件次第で304を返すようにする
    let x = GLOBAL_FILE.parse_file_as_json()
        .map(|x|
            (
                ArticleIdSet(
                    x.data.iter()
                        .filter(f)
                        .map(|(x, _)| x)
                        .cloned()
                        .collect::<HashSet<_>>()
                ),
                // oldest creation
                x.data.iter()
                    .filter(f)
                    .min_by_key(|(_, x)| x.created_at)
                    .map(|(_, x)| x.created_at),
                // newest update
                x.data.iter()
                    .filter(f)
                    .max_by_key(|(_, x)| x.updated_at)
                    .map(|(_, x)| x.updated_at),
            )
        );

    match x {
        Ok((l, old_cre, new_upd)) =>
            EndpointRepresentationCompiler::from_value(
                ArticleIdCollectionResponseRepr(OwnedMetadata {
                    metadata: ArticleIdSetMetadata {
                        oldest_created_at: old_cre,
                        newest_updated_at: new_upd,
                    },
                    data: l
                })
            ).into_json()
                .map_body(|_, y| serde_json::to_string(&y).expect(""))
                .map_into_boxed_body(),
        Err(e) => EndpointRepresentationCompiler::from_value(
            InternalErrorExposedRepr(Box::new(e))
        ).into_json()
            .map_body(|_, y| serde_json::to_string(&y).expect(""))
            .map_into_boxed_body()
    }
}
