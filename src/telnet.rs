mod ansi;
mod state;
mod stream;
mod response;
mod process_command;
mod repository;

use std::sync::{Arc, Mutex};
use actix_web::web::BytesMut;
use anyhow::{Result, Context as _, bail};
use log::debug;
use telnet_codec::{TelnetCodec, TelnetEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use crate::telnet::process_command::process_command;
use crate::telnet::state::{CONNECTION_POOL, TemporaryStatus};
use crate::telnet::stream::{write_text_to_stream};

#[allow(clippy::too_many_lines, clippy::future_not_send, clippy::module_name_repetitions)]
pub async fn telnet_server_service(stream: TcpStream) -> Result<()> {
    let stream = Arc::new(Mutex::new(stream));
    let get_stream = || {
        debug!("getting stream lock");
        let r = stream.lock().unwrap();
        debug!("ok");
        r
    };

    let addr = get_stream().peer_addr().context("get peer addr")?;
    debug!("welcome, {}", &addr);
    {
        // don't mutex lock live long
        CONNECTION_POOL.add(addr, TemporaryStatus::default());
    }

    let prompt = || async {
        if CONNECTION_POOL.get_and_pick(addr, |ts| ts.prompt).unwrap() {
            write_text_to_stream(&mut get_stream(), "toy-blog telnet> ").await;
        }
    };

    let process_command = |parts: Vec<String>| {
        debug!("process command");

        async move {
            let stream = &mut get_stream();
            process_command(parts, addr, stream).await;
        }
    };

    let buf = Arc::new(Mutex::new(BytesMut::with_capacity(4096)));

    let try_process_current_buffer = || {
        let mut cloned = buf.lock().unwrap();
        let mut tc = TelnetCodec::new(8192);

        async move {
            while let Some(telnet_packet) = tc.decode(&mut cloned).unwrap() {
                // continue = ignore
                // break = more bytes needed
                match telnet_packet {
                    TelnetEvent::Negotiate(a, b) => {
                        // DO
                        if a == 253 {
                            match b {
                                6 => {
                                    // explicit time sync (RFC 860)
                                    // but we do not sync due to rack of knowledge about Telnet
                                }
                                _ => {
                                    debug!("negotiate: it is unknown, or unimplemented option: {b}");
                                }
                            }
                        }
                        break
                    },
                    TelnetEvent::SubNegotiate(a, bytes) => {
                        debug!("sub negotiate: {a}, {bytes:?}");
                        continue
                    },
                    TelnetEvent::Data(bytes) => {
                        match String::from_utf8(bytes.to_vec()) {
                            Ok(s) => {
                                if let Some(s) = s.strip_suffix("\r\n") {
                                    let parts = s.splitn(2, ' ')
                                        .map(ToString::to_string)
                                        .filter(|a| !a.is_empty())
                                        .collect();
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
                        if code == 244 {
                            debug!("ctrl-C was received");
                        }
                        break
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

    loop {
        debug!("read");

        prompt().await;

        match read_buf().await {
            // disconnected
            Ok(0) => break,

            // write bytes back to stream
            Ok(_bytes_read) => {
                debug!("data recv");
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
