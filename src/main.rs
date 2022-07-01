#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod backend;
mod extension;

// TODO: telnetサポートしたら面白いんじゃね？ - @yanorei32

use actix_web::{App, HttpServer, web, get, Responder};
use actix_web::middleware::Logger;
use actix_web::web::{scope as prefixed_service};
use anyhow::{Result, Context as _};
use fern::colors::ColoredLevelConfig;

fn setup_logger() -> Result<()> {
    let mut colors = ColoredLevelConfig::new();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[get("/")]
async fn hello() -> impl Responder {
    "Hello, World!"
}

#[actix_web::main]
async fn main() -> Result<()> {
    setup_logger().unwrap_or_default();
    use crate::backend::api::{article, user};

    let server = HttpServer::new(|| {
        App::new()
            // TODO: postponed
            //   - GET    /api/user/current
            //   - POST   /api/user/token/create
            //   - GET    /api/user/token/list
            //   - DELETE /api/user/token/delete
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
                        /*
                        prefixed_service("/user")
                            .service(user::login),

                         */
                    )
                )
            )
            .service(hello)
    });


    server
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
        .context("while running server")?;

    Ok(())
}
