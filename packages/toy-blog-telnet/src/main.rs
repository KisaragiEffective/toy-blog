#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod service;

use std::error::Error;
use std::net::TcpStream;
use actix_server::Server;
use actix_service::fn_service;
use clap::Parser;
use url::Url;
use crate::service::telnet::telnet_server_service;

#[derive(Parser)]
enum Args {
    Run {
        /// Specify port.
        /// The alias `--telnet-port` is deprecated. It will be removed in next major release.
        /// Please use `--port` instead.
        #[clap(long, long = "telnet-port")]
        port: u16,
        /// Specify host.
        /// The alias `telnet-host` is deprecated. It will be removed in next major release.
        /// Please use `--host` instead.
        #[clap(long, long = "telnet-host")]
        host: String,
        api_endpoint: Url,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>>{
    let args: Args = Args::parse();

    match args {
        Args::Run {
            port,
            host,
            api_endpoint,
        } => {
            tokio::spawn({
                Server::build()
                    .bind("echo", (host, port), move || {
                        fn_service(move |stream: TcpStream| {
                            telnet_server_service(stream)
                        })
                    })?
                    .run()
            });

            Ok(())
        }
    }
}
