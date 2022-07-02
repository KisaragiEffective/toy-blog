#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod backend;
mod extension;

// TODO: telnetサポートしたら面白いんじゃね？ - @yanorei32

use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::string::FromUtf8Error;
use actix_web::{App, HttpServer};
use actix_web::dev::{fn_service, Server};
use actix_web::middleware::Logger;

use actix_web::web::{BytesMut, scope as prefixed_service};
use anyhow::{Result, Context as _, bail};
use fern::colors::ColoredLevelConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use futures_util::{TryFutureExt, TryStreamExt};
use log::{debug, info};
use telnet_codec::{TelnetCodec, TelnetEvent};
use tokio_util::codec::Decoder;

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

            .wrap(Logger::new(r#"%a(CF '%{CF-Connecting-IP}i') %t "%r" %s "%{Referer}i" "%{User-Agent}i" "#))
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
                fn_service(move |mut stream: TcpStream| {
                    async move {
                        let addr = stream.peer_addr().context("get peer addr")?;
                        debug!("welcome, {}", addr);
                        stream.write_all(b"Welcome!\r\n").await.unwrap();
                        let mut tc = telnet_codec::TelnetCodec::new(8192);
                        'connection_loop: loop {
                            let mut buf = BytesMut::new();
                            prompt(&mut stream, addr).await;

                            let constructed = once_cell::unsync::OnceCell::new();

                            'read: loop {
                                debug!("read");
                                match stream.read_buf(&mut buf).await {
                                    Ok(bytes_read) => {
                                        debug!("bytes read: {bytes_read}");
                                        debug!("current raw buffer: {bytes:x?}", bytes = buf.as_ref());

                                        if bytes_read == 0 {
                                            debug!("Socket was closed. Bye-bye {addr}!");
                                            break 'connection_loop
                                        }

                                        let parsed_telnet_codec = tc.decode(&mut buf).expect("死");

                                        if let Some(parsed_codec) = parsed_telnet_codec {
                                            debug!("telnet message candidate");
                                            if let TelnetEvent::Data(bytes) = &parsed_codec {
                                                debug!("candidate is data. dump: {bytes:x?} checking if terminated by a CRLF", bytes = &buf);
                                                // each command must be terminated by CRLF in this connection
                                                if bytes.ends_with(b"\r\n") {
                                                    debug!("OK");
                                                    constructed.set(parsed_codec).unwrap();
                                                    break 'read
                                                }

                                                debug!("NG");
                                            } else {
                                                debug!("candidate is not data, pass-through");
                                                constructed.set(parsed_codec).unwrap();
                                                break 'read
                                            }
                                        }
                                    }

                                    Err(err) => {
                                        eprintln!("Stream Error: {:?}", err);
                                        bail!("oops, stream error");
                                    }
                                }
                            }

                            let mut outer_constructed = constructed.get().cloned();
                            debug!("raw byte buffer: {raw:?}", raw = &buf);
                            debug!("received telnet message: {constructed:?}");
                            while let Some(constructed) = outer_constructed {
                                match constructed {
                                    TelnetEvent::Negotiate(a, b) => {}
                                    TelnetEvent::SubNegotiate(c, d) => {}
                                    TelnetEvent::Data(raw_bytes) => {
                                        let read_command = match String::from_utf8(raw_bytes.to_vec()) {
                                            Ok(s) => s,
                                            Err(e) => {
                                                debug!("command is invalid UTF-8 sequence. Error: {e}");
                                                stream.write_all(b"Command must be represented in valid UTF-8 sequence. Please resend.").await.unwrap();
                                                buf.clear();
                                                continue 'connection_loop
                                            }
                                        };

                                        debug!("transmit");
                                        stream.write_all(format!("I got: {b:?}\r\n", b = read_command.as_bytes()).as_bytes()).await.unwrap();
                                    }
                                    TelnetEvent::Command(command) => {
                                        if command == 244 {
                                            stream.write_all(b"Bye-bye! (Ctrl-C)").await.unwrap();
                                            disconnect(&mut stream).await.unwrap();
                                        }
                                    }
                                }
                                let a = tc.decode(&mut buf)?;
                                outer_constructed = a;
                            }


                            // DO NOT REMOVE
                            buf.clear();
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
