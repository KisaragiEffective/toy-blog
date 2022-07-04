#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod backend;
mod extension;

// TODO: telnetサポートしたら面白いんじゃね？ - @yanorei32

use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;

use actix_web::web::{scope as prefixed_service};
use actix_web_httpauth::extractors::bearer::{Config as BearerAuthConfig};
use anyhow::{Result, Context as _};
use fern::colors::ColoredLevelConfig;
use crate::backend::api::article;

fn setup_logger() -> Result<()> {
    let colors = ColoredLevelConfig::new();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ));
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<()> {
    setup_logger().unwrap_or_default();

    let server = HttpServer::new(|| {
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
                        article::list,
                    )
                )
            )
            .app_data(
                BearerAuthConfig::default()
                    .realm("Perform write operation")
                    .scope("article:write"),
            )
            .wrap(Logger::new(r#"%a(CF '%{CF-Connecting-IP}i') %t "%r" %s "%{Referer}i" "%{User-Agent}i" "#))
    });


    server
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
        .context("while running server")?;

    Ok(())
}
