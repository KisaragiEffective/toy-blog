use std::path::PathBuf;
use clap::{Parser, Subcommand};
use toy_blog_endpoint_model::ArticleId;

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Commands
}

#[derive(Subcommand)]
pub enum Commands {
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
    Version {
        #[clap(long)]
        plain: bool,
    }
}
