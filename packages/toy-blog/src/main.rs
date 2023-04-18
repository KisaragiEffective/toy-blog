#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod extension;
mod service;

use std::fs::File;
use std::io::{BufReader, Read, stdin};
use std::path::PathBuf;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;

use actix_web::web::scope as prefixed_service;
use anyhow::{bail, Context as _, Result};
use actix_web_httpauth::extractors::bearer::Config as BearerAuthConfig;
use clap::{Parser, Subcommand};
use fern::colors::ColoredLevelConfig;
use log::{debug, info};
use service::http::auth::WRITE_TOKEN;

use crate::service::http::api::{article, meta};
use crate::service::http::cors::middleware_factory as cors_middleware_factory;
use toy_blog_endpoint_model::ArticleId;
use crate::service::http::api::list::{article_id_list, article_id_list_by_year, article_id_list_by_year_and_month};
use crate::service::http::repository::GLOBAL_FILE;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Commands
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[clap(long)]
        http_port: u16,
        #[clap(long)]
        http_host: String,
        #[clap(long = "cloudflare")]
        cloudflare_support: bool,
        /// DEPRECATED, It will be removed in next major version. This switch is no-op.
        #[clap(long)]
        read_bearer_token_from_stdin: bool,
    },
    Import {
        #[clap(long)]
        file_path: PathBuf,
        #[clap(long)]
        article_id: ArticleId,
    },
}

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
    let args: Args = Args::parse();
    match args.subcommand {
        Commands::Run {
            http_port,
            http_host,
            cloudflare_support,
            read_bearer_token_from_stdin: _
        } => {
            let bearer_token = {
                let mut buf = String::new();
                stdin().read_line(&mut buf).expect("failed to read from stdin");
                buf.trim_end().to_string()
            };

            WRITE_TOKEN.set(bearer_token).unwrap();

            let http_server = HttpServer::new(move || {
                let logger_format = if cloudflare_support {
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
                    .wrap(cors_middleware_factory())
            });

        http_server
                    .bind((http_host, http_port))?
                    .run()
                    .await
                    .context("while running server")?;

                Ok(())
            }
        Commands::Import { file_path, article_id } => {
            if !file_path.exists() {
                bail!("You can not import non-existent file")
            }

            if !file_path.is_file() {
                // TODO: /dev/stdin is not supported by this method
                debug!("is_dir: {}", file_path.is_dir());
                debug!("is_symlink: {}", file_path.is_symlink());
                debug!("metadata: {:?}", file_path.metadata()?);
                bail!("Non-file paths are not supported")
            }

            let content = {
                let mut fd = BufReader::new(File::open(file_path)?);
                let mut buf = vec![];
                fd.read_to_end(&mut buf)?;
                String::from_utf8(buf)
            };

            match content {
                Ok(content) => {
                    GLOBAL_FILE.create_entry(&article_id, content).await?;
                    info!("Successfully imported as {article_id}.");
                    Ok(())
                }
                Err(err) => {
                    bail!("The file is not UTF-8: {err}\
                    Please review following list:\
                    - The file is not binary\
                    - The text is encoded with UTF-8\
                    Especially, importing Shift-JIS texts are NOT supported.")
                }
            }
        }
    }
}
