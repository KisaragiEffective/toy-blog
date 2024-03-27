use std::future::{Future, ready};

use actix_web::{get, Responder};
use actix_web::web::Path;
use chrono::Datelike;

use toy_blog_endpoint_model::{AnnoDominiYear, Article, ArticleId, ArticleIdSet, ArticleIdSetMetadata, OneOriginTwoDigitsMonth, OwnedMetadata, Visibility};

use crate::service::persistence::ArticleRepository;
use crate::service::rest::exposed_representation_format::{ArticleIdCollectionResponseRepr, EndpointRepresentationCompiler, MaybeNotModified, ReportLastModofied};
use crate::service::rest::header::IfModifiedSince;
use crate::service::rest::repository::GLOBAL_FILE;

fn compute_and_filter_out(
    article_repository: &ArticleRepository,
    if_modified_since: Option<IfModifiedSince>,
    additional_filter: impl Clone + Fn(&&(ArticleId, Article)) -> bool
) -> ArticleIdCollectionResponseRepr {
    let x = article_repository.entries();
    let only_public = |x: &&(ArticleId, Article)| x.1.visibility == Visibility::Public;
    let ret_304;
    let latest_updated = x.iter()
        .filter(only_public)
        .filter(additional_filter.clone())
        .max_by_key(|r| r.1.updated_at)
        .map(|x| &x.1).map(|x| x.updated_at);

    if let Some(if_modified_since) = if_modified_since {
        let if_unmodified_since = if_modified_since.0.0;
        ret_304 = latest_updated.is_some_and(|d| d >= if_unmodified_since);
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
                latest_updated: latest_updated.map(std::convert::Into::into)
            },
            is_modified: ret_304,
        }
    )
}

const ONCE_CELL_INITIALIZATION_ERROR: &str = "must be fully initialized";

#[get("/article")]
#[allow(clippy::unused_async)]
pub fn article_id_list(if_modified_since: Option<IfModifiedSince>) -> impl Future<Output = impl Responder> {
    let v = EndpointRepresentationCompiler::from_value(
        article_id_list0(GLOBAL_FILE.get().expect(ONCE_CELL_INITIALIZATION_ERROR), if_modified_since)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body();

    ready(v)
}

fn article_id_list0(repo: &ArticleRepository, if_modified_since: Option<IfModifiedSince>) -> ArticleIdCollectionResponseRepr {
    compute_and_filter_out(repo, if_modified_since, |_| true)
}

#[get("/article/{year}")]
#[allow(clippy::unused_async)]
pub fn article_id_list_by_year(path: Path<AnnoDominiYear>, if_modified_since: Option<IfModifiedSince>) -> impl Future<Output = impl Responder> {
    let v = EndpointRepresentationCompiler::from_value(
        article_id_list_by_year0(GLOBAL_FILE.get().expect(ONCE_CELL_INITIALIZATION_ERROR), path.into_inner(), if_modified_since)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body();

    ready(v)
}

fn article_id_list_by_year0(repo: &ArticleRepository, path: AnnoDominiYear, if_modified_since: Option<IfModifiedSince>) -> ArticleIdCollectionResponseRepr {
    let year = path.into_inner();
    compute_and_filter_out(repo, if_modified_since, |x| x.1.created_at.year() as u32 == year)
}

#[get("/article/{year}/{month}")]
pub fn article_id_list_by_year_and_month(
    path: Path<(AnnoDominiYear, OneOriginTwoDigitsMonth)>, if_modified_since: Option<IfModifiedSince>
) -> impl Future<Output = impl Responder> {
    let v = EndpointRepresentationCompiler::from_value(
        article_id_list_by_year_and_month0(GLOBAL_FILE.get().expect(ONCE_CELL_INITIALIZATION_ERROR), path.into_inner(), if_modified_since)
    ).into_json()
        .map_body(|_, y| serde_json::to_string(&y).expect(""))
        .map_into_boxed_body();

    ready(v)
}

fn article_id_list_by_year_and_month0(repo: &ArticleRepository, path: (AnnoDominiYear, OneOriginTwoDigitsMonth), if_modified_since: Option<IfModifiedSince>) 
    -> ArticleIdCollectionResponseRepr {
    let (year, month) = path;
    let year = year.into_inner();
    let month = month.into_inner();
    compute_and_filter_out(repo, if_modified_since, |x| x.1.created_at.year() as u32 == year && x.1.created_at.month() as u8 == month)
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Local};

    use toy_blog_endpoint_model::{AnnoDominiYear, ArticleId, OneOriginTwoDigitsMonth, Visibility};

    use crate::service::persistence::ArticleRepository;
    use crate::service::rest::api::list::{article_id_list0, article_id_list_by_year0, article_id_list_by_year_and_month0};

    #[test]
    fn do_not_leak() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let m = tempfile::NamedTempFile::new().expect("failed to initialize temporary file");
                ArticleRepository::init(m.path());
                let a = ArticleRepository::new(m.path()).await;
                {
                    let aa = ArticleId::new("123".to_string());
                    a.create_entry(&aa, "12345".to_string(), Visibility::Private).await.unwrap();
                    let ac = article_id_list0(&a, None);
                    let m = ac.0.inner.inner.data.0.get(&aa);
                    assert!(m.is_none());
                }
                {
                    let aa = ArticleId::new("1234".to_string());
                    a.create_entry(&aa, "123456".to_string(), Visibility::Restricted).await.unwrap();
                    let ac = article_id_list0(&a, None);
                    let m = ac.0.inner.inner.data.0.get(&aa);
                    assert!(m.is_none());
                }
            });
    }

    #[test]
    fn do_not_leak_by_year() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let m = tempfile::NamedTempFile::new().expect("failed to initialize temporary file");
                ArticleRepository::init(m.path());
                let a = ArticleRepository::new(m.path()).await;
                {
                    let aa = ArticleId::new("123".to_string());
                    a.create_entry(&aa, "12345".to_string(), Visibility::Private).await.unwrap();
                    let ac = article_id_list_by_year0(&a, AnnoDominiYear::try_from(Local::now().year() as u32).unwrap(), None);
                    let m = ac.0.inner.inner.data.0.get(&aa);
                    assert!(m.is_none());
                }
                {
                    let aa = ArticleId::new("1234".to_string());
                    a.create_entry(&aa, "123456".to_string(), Visibility::Restricted).await.unwrap();
                    let ac = article_id_list_by_year0(&a, AnnoDominiYear::try_from(Local::now().year() as u32).unwrap(), None);
                    let m = ac.0.inner.inner.data.0.get(&aa);
                    assert!(m.is_none());
                }
            });
    }

    #[test]
    fn do_not_leak_by_year_and_month() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let m = tempfile::NamedTempFile::new().expect("failed to initialize temporary file");
                ArticleRepository::init(m.path());
                let a = ArticleRepository::new(m.path()).await;
                {
                    let aa = ArticleId::new("123".to_string());
                    a.create_entry(&aa, "12345".to_string(), Visibility::Private).await.unwrap();
                    let now = Local::now();
                    let ac = article_id_list_by_year_and_month0(
                        &a, (
                            AnnoDominiYear::try_from(now.year() as u32).unwrap(),
                            OneOriginTwoDigitsMonth::try_from(now.month() as u8).unwrap()
                        ), None
                    );
                    let a = ac.0.inner.inner.data.0.get(&aa);
                    assert!(a.is_none());
                }
                {
                    let aa = ArticleId::new("1235".to_string());
                    a.create_entry(&aa, "123456".to_string(), Visibility::Restricted).await.unwrap();
                    let now = Local::now();
                    let ac = article_id_list_by_year_and_month0(
                        &a, (
                            AnnoDominiYear::try_from(now.year() as u32).unwrap(),
                            OneOriginTwoDigitsMonth::try_from(now.month() as u8).unwrap()
                        ), None
                    );
                    let a = ac.0.inner.inner.data.0.get(&aa);
                    assert!(a.is_none());
                }
            });
    }
}