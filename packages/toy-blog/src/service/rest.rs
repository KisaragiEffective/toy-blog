pub mod api;
pub mod cors;
pub mod repository;
pub mod auth;
pub mod exposed_representation_format;

use std::fs::File;
use std::io::stdin;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use anyhow::Context;
use log::info;
use serde_json::Value;
use inner_no_leak::ComposeInternalError;
use crate::service::persistence::ArticleRepository;
use crate::service::rest::api::{article, meta};
use crate::service::rest::api::list::{article_id_list, article_id_list_by_year, article_id_list_by_year_and_month};
use crate::service::rest::auth::WRITE_TOKEN;
use crate::service::rest::repository::GLOBAL_FILE;
use actix_web::web::scope as prefixed_service;
use actix_web_httpauth::extractors::bearer::Config as BearerAuthConfig;

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

pub async fn boot_http_server(port: u16, host: &str, proxied_by_cloudflare: bool) -> Result<(), anyhow::Error> {
    let bearer_token = {
        let mut buf = String::new();
        stdin().read_line(&mut buf).expect("failed to read from stdin");
        buf.trim_end().to_string()
    };

    const PATH: &str = "data/article.json";

    // migration

    {
        #[allow(unused_qualifications)]
            let migrated_data = crate::migration::migrate_article_repr(
            serde_json::from_reader::<_, Value>(File::open(PATH).expect("failed to read existing config"))
                .expect("failed to deserialize config")
        );

        info!("migrated");

        serde_json::to_writer(
            File::options().write(true).truncate(true).open(PATH).expect("failed to write over existing config"),
            &migrated_data
        )
            .expect("failed to serialize config");
    }

    GLOBAL_FILE.set(ArticleRepository::new(PATH).await).expect("unreachable!");

    WRITE_TOKEN.set(bearer_token).unwrap();

    let http_server = HttpServer::new(move || {
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
            .wrap(Logger::new(logger_format))
            .wrap(crate::service::rest::cors::middleware_factory())
    });

    println!("running!");
    http_server
        .bind((host, port))?
        .run()
        .await
        .context("while running server")?;

    Ok(())
}
