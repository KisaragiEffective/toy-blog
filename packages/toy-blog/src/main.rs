#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// This fires on HttpRequest, which is not FP.
// But we don't want to be triggered because service function often refers it.
#![allow(clippy::future_not_send)]

mod extension;
mod service;
mod migration;


use std::io::Read;

use anyhow::Result;
use clap::Parser;
use fern::colors::ColoredLevelConfig;

use crate::service::cli::{Args, Commands};
use crate::service::rest::repository::GLOBAL_FILE;

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
            crate::service::rest::boot_http_server(http_port, &http_host, cloudflare_support).await
        }
        Commands::Import { file_path, article_id } => {
            crate::service::import::import(&file_path, &article_id).await
        }
        Commands::Version { plain } => {
            const VERSION: &str = env!("CARGO_PKG_VERSION");
            const NAME: &str = env!("CARGO_PKG_NAME");
            if plain {
                println!("{VERSION}");
            } else {
                println!("{NAME} {VERSION}");
            }

            Ok(())
        }
    }
}
