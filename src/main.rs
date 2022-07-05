#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod backend;
mod extension;

// TODO: telnetサポートしたら面白いんじゃね？ - @yanorei32

use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::string::FromUtf8Error;
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::time::Duration;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use actix_web::{App, HttpServer};
use actix_web::dev::{fn_service, Server};
use actix_web::middleware::Logger;

use actix_web::web::{BytesMut, scope as prefixed_service};
use anyhow::{Result, Context as _, bail};
use actix_web_httpauth::extractors::bearer::{Config as BearerAuthConfig};
use clap::{Parser, Subcommand};
use fern::colors::ColoredLevelConfig;
use log::{debug, info};
use once_cell::sync::OnceCell;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use futures_util::{TryFutureExt, TryStreamExt};
use mio::{Events, Poll};
use telnet_codec::{TelnetCodec, TelnetEvent};
use tokio_util::codec::Decoder;

use crate::backend::api::article;
use crate::backend::cors::{middleware_factory as cors_middleware_factory};
use crate::backend::persistence::model::ArticleId;
use crate::backend::repository::GLOBAL_FILE;

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
    setup_logger().unwrap_or_default();
    let args: Args = Args::parse();
    match args.subcommand {
        Commands::Run { bearer_token } => {
            GIVEN_TOKEN.set(bearer_token).unwrap();

            let http_server = HttpServer::new(|| {
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

    tokio::spawn({
        async fn prompt(stream: &mut TcpStream, addr: SocketAddr) {
            match stream.write_all(b"toy-blog telnet > ").await {
                Ok(_) => {}
                Err(e) => {
                    match e.kind() {
                        ErrorKind::BrokenPipe => {
                            debug!("Connection is closed by remote client ({addr}). Closing pipe.");
                            // This will result in NotConnected, anyway ignores that
                            disconnect(stream).await.unwrap_or_default();
                        }
                        // "rethrow"
                        _ => Err(e).unwrap()
                    }
                }
            }
        }

        async fn disconnect(stream: &mut TcpStream) -> std::io::Result<()> {
            stream.shutdown().await
        }

        Server::build()
            .bind("echo", ("127.0.0.1", 23112), move || {
                fn_service(move |stream: TcpStream| {
                    async move {
                        let stream = Arc::new(Mutex::new(stream));
                        let get_stream = || {
                            debug!("getting stream lock");
                            let r = stream.lock().unwrap();
                            debug!("ok");
                            r
                        };
                        let addr = get_stream().peer_addr().context("get peer addr")?;
                        debug!("welcome, {}", addr);
                        get_stream().write_all(b"Welcome!\r\n").await.unwrap();
                        get_stream().write_all(b"Don't use telnet(1)!\r\n").await.unwrap();
                        let mut buf = Arc::new(Mutex::new(BytesMut::with_capacity(4096)));
                        let mut size = 0;
                        let process_command = |parts: Vec<String>| {
                            debug!("process command");

                            async move {
                                let mut stream = get_stream();
                                match parts.len() {
                                    0 => {
                                    },
                                    1 => {
                                        let command = parts[0].as_str();
                                        match command {
                                            "HELP" => {
                                                stream.write_all(b"TODO: Well documented help").await.unwrap();
                                            }
                                            _ => {
                                                stream.write_all(b"Unknown command. Please type HELP to display help.").await.unwrap();
                                            }
                                        }
                                    },
                                    2 => {
                                        let (command, params) = (parts[0].as_str(), parts[1].as_str());
                                    },
                                    _ => unreachable!()
                                }
                            }
                        };
                        let try_process_current_buffer = || {
                            let mut cloned = buf.lock().unwrap();
                            let mut tc = TelnetCodec::new(8192);

                            async move {
                                while let Some(telnet_packet) = tc.decode(&mut cloned).unwrap() {
                                    // continue = ignore
                                    // break = more bytes needed
                                    match telnet_packet {
                                        TelnetEvent::Negotiate(a, b) => {
                                            debug!("negotiate, noun: {a}, option: {b}");
                                            continue
                                        },
                                        TelnetEvent::SubNegotiate(a, bytes) => {
                                            debug!("sub negotiate: {a}, {bytes:?}");
                                            continue
                                        },
                                        TelnetEvent::Data(bytes) => {
                                            match String::from_utf8(bytes.to_vec()) {
                                                Ok(s) => {
                                                    if let Some(s) = s.strip_suffix("\r\n") {
                                                        let parts = s.splitn(2, ' ').map(ToString::to_string).collect();
                                                        process_command(parts).await;
                                                    } else {
                                                        debug!("incomplete text command, awaiting further bytes");
                                                        break
                                                    }
                                                }
                                                Err(e) => {
                                                    let mut stream = get_stream();
                                                    stream.write_all(b"Error, this is not an UTF-8.\r\n").await.unwrap();
                                                    stream.write_all(format!("Description: {e}\r\n").as_bytes()).await.unwrap();
                                                    stream.write_all(b"Maybe more bytes needed.\r\n").await.unwrap();
                                                    break
                                                }
                                            }
                                        }
                                        TelnetEvent::Command(code) => {
                                            debug!("command: {code}");
                                            continue
                                        }
                                    }
                                }
                            }
                        };

                        // This function should not be inlined; it causes an dead-lock.
                        let read_buf = || {
                            async {
                                get_stream().read_buf(&mut *buf.lock().unwrap()).await
                            }
                        };

                        let output = |s: &str| {
                            let s = s.to_string().into_boxed_str();
                            async move {
                                get_stream().write_all(s.as_bytes()).await.unwrap();
                            }
                        };
                        'connection_loop: loop {
                            debug!("read");
                            output("toy blog> ").await;

                            match read_buf().await {
                                // disconnected
                                Ok(0) => break,

                                // write bytes back to stream
                                Ok(_bytes_read) => {
                                    debug!("data recv");
                                    // get_stream().write_all(b"human readable: ").await.unwrap();
                                    // get_stream().write_all(&buf.lock().unwrap()).await.unwrap();
                                    // get_stream().write_all(format!("raw representation: {bytes:x?}\n", bytes = &buf.lock().unwrap()).as_bytes()).await.unwrap();
                                    // get_stream().flush().await.unwrap();
                                    try_process_current_buffer().await;
                                    buf.lock().unwrap().clear();
                                }

                                Err(err) => {
                                    eprintln!("Stream Error: {:?}", err);
                                    bail!("stream err");
                                }
                            }
                        }

                        Ok(())
                    }
                })
            })?
            .run()
    });

    http_server
                .bind(("127.0.0.1", 8080))?
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
