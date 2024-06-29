mod api;
mod cors;
pub(in crate::service) mod repository;
mod auth;
mod exposed_representation_format;
mod header;

use std::fs::File;
use std::io::stdin;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use actix_web::{App, HttpResponseBuilder, HttpServer};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use anyhow::Context;
use log::info;
use serde_json::Value;
use inner_no_leak::ComposeInternalError;
use crate::service::persistence::ArticleRepository;
use crate::service::rest::api::{article, meta};
use crate::service::rest::api::list::{article_id_list, article_id_list_by_year, article_id_list_by_year_and_month};
use crate::service::rest::auth::WRITE_TOKEN;
use crate::service::rest::repository::GLOBAL_ARTICLE_REPOSITORY;
use actix_web::web::scope as prefixed_service;
use actix_web_httpauth::extractors::bearer::Config as BearerAuthConfig;
use futures_util::future::LocalBoxFuture;
use futures_util::FutureExt;

mod inner_no_leak {
    use std::error::Error;
    use thiserror::Error;

    pub type ComposeInternalError<T> = Result<T, UnhandledError>;

    #[derive(Error, Debug)]
    #[error("Internal error: {_0}")]
    pub struct UnhandledError(pub Box<dyn Error>);

    impl UnhandledError {
        pub fn new<E: Error + 'static>(error: E) -> Self {
            Self(Box::new(error) as _)
        }
    }
}

async fn migrate_and_load(path: impl AsRef<Path>) -> ArticleRepository {
    ArticleRepository::create_default_file_if_absent(path.as_ref());
    {
        #[allow(unused_qualifications)]
            let migrated_data = crate::migration::migrate_article_repr(
            serde_json::from_reader::<_, Value>(File::open(path.as_ref()).expect("failed to read existing config"))
                .expect("failed to deserialize config")
        );

        info!("migrated");

        serde_json::to_writer(
            File::options().write(true).truncate(true).open(path.as_ref()).expect("failed to write over existing config"),
            &migrated_data
        )
            .expect("failed to serialize config");
    }

    ArticleRepository::new(path.as_ref()).await
}

pub async fn boot_http_server(port: u16, host: &str, proxied_by_cloudflare: bool) -> Result<(), anyhow::Error> {
    const PATH: &str = "data/article.json";

    let bearer_token = {
        let mut buf = String::new();
        stdin().read_line(&mut buf).expect("failed to read from stdin");
        buf.trim_end().to_string()
    };
    // migration

    let repo = migrate_and_load(PATH).await;
    WRITE_TOKEN.set(bearer_token).unwrap();

    // TODO: AppやHttpServerの型変数が記述できないため関数にくくり出せない
    GLOBAL_ARTICLE_REPOSITORY.set(repo).expect("unreachable!");
    let http_server_closure = |proxied_by_cloudflare| {
        let logger_format = if proxied_by_cloudflare {
            r#"%a (CF '%{CF-Connecting-IP}i') %t "%r" %s "%{Referer}i" "%{User-Agent}i" "#
        } else {
            r#"%a %t "%r" %s "%{Referer}i" "%{User-Agent}i" "#
        };

        App::new()
            .service(prefixed_service("/api")
                .service(
                    (
                        prefixed_service("/article")
                            .service(
                                (
                                    article::create,
                                    article::fetch,
                                    article::update,
                                    article::remove,
                                    article::update_visibility,
                                )
                            ),
                        prefixed_service("/meta")
                            .service(meta::change_id),
                        prefixed_service("/list")
                            .service(article_id_list)
                            .service(article_id_list_by_year)
                            .service(article_id_list_by_year_and_month)
                    )
                )
            )
            .app_data(
                BearerAuthConfig::default()
                    .realm("Perform write operation")
                    .scope("article:write"),
            )
            .wrap_fn(move |req, srv| {
                let cloudflare_support = proxied_by_cloudflare;

                const HATENA_BOOKMARK_CRAWLER: Ipv4Addr = Ipv4Addr::new(133, 242, 243, 6);
                let extract_real_ip = |req: &ServiceRequest, cloudflare_support: bool| {
                    if cloudflare_support {
                        req.headers().get("CF-Connecting-IP")?.to_str().ok()?.parse::<IpAddr>().ok()
                    } else {
                        req.peer_addr().map(|x| x.ip())
                    }
                };

                if extract_real_ip(&req, cloudflare_support).is_some_and(|x| x == HATENA_BOOKMARK_CRAWLER) {
                    Box::pin(async {
                        Ok(ServiceResponse::new(req.into_parts().0, HttpResponseBuilder::new(StatusCode::FORBIDDEN).body("Forbidden")))
                    }) as LocalBoxFuture<Result<ServiceResponse, actix_web::Error>>
                } else {
                    use actix_web::dev::Service;

                    Box::pin(srv.call(req).map(|x| x.map(|y| y.map_into_boxed_body())))  
                        as LocalBoxFuture<Result<ServiceResponse, actix_web::Error>>
                }
            })
            .wrap(Logger::new(logger_format))
            .wrap(crate::service::rest::cors::middleware_factory())
    };
    
    let http_server = HttpServer::new(move || http_server_closure(proxied_by_cloudflare));

    println!("running!");
    http_server
        .bind((host, port))?
        .run()
        .await
        .context("while running server")?;

    Ok(())
}
