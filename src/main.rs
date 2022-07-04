#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod backend;
mod extension;

// TODO: telnetサポートしたら面白いんじゃね？ - @yanorei32

use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::PathBuf;
use std::string::FromUtf8Error;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;

use actix_web::web::{scope as prefixed_service};
use actix_web_httpauth::extractors::bearer::{Config as BearerAuthConfig};
use anyhow::{Result, Context as _, bail};
use clap::{Parser, Subcommand};
use fern::colors::ColoredLevelConfig;
use once_cell::sync::OnceCell;
use crate::backend::api::article;
use crate::backend::cors::{middleware_factory as cors_middleware_factory};
use crate::backend::persistence::model::ArticleId;

static GIVEN_TOKEN: OnceCell<String> = OnceCell::new();

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Commands
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[clap(long)]
        bearer_token: String,
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
    let args: Args = Args::parse();
    match args.subcommand {
        Commands::Run { bearer_token } => {
            GIVEN_TOKEN.set(bearer_token).unwrap();
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
                    .wrap(cors_middleware_factory())
            });


            server
                .bind(("127.0.0.1", 8080))?
                .run()
                .await
                .context("while running server")?;

            Ok(())
        }
        Commands::Import { file_path } => {
            if !file_path.exists() {
                bail!("You can not import non-existent file")
            }

            if !file_path.is_file() {
                bail!("Non-file paths are not supported")
            }

            let string = {
                let mut fd = BufReader::new(File::open(file_path)?);
                let mut buf = vec![];
                fd.read_to_end(&mut buf)?;
                String::from_utf8(buf)
            };

            match string {
                Ok(string) => {
                    crate::backend::repository::GLOBAL_FILE.create_entry()
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
