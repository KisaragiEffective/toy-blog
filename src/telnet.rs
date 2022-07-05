use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use actix_web::web::BytesMut;
use anyhow::{Result, Context as _, bail};
use log::debug;
use once_cell::sync::Lazy;
use telnet_codec::{TelnetCodec, TelnetEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use crate::{GLOBAL_FILE, ListOperationScheme};

static CONNECTION_POOL: Lazy<Arc<Mutex<HashMap<SocketAddr, ConnectionState>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Default)]
struct ConnectionState {
    prompt: bool,
}

#[allow(unused_variables)] // false-positive on IntelliJ
async fn writeln_text_to_stream<'a, 'b: 'a>(stream: &'a mut TcpStream, text: &'b str) {
    write_text_to_stream(stream, format!("{text}\r\n").as_str()).await;
}

async fn write_text_to_stream<'a, 'b: 'a>(stream: &'a mut TcpStream, text: &'b str) {
    match stream.write_all(text.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            match e.kind() {
                ErrorKind::BrokenPipe => {
                    debug!("Connection is closed by remote client ({addr}). Closing pipe.", addr = stream.peer_addr().unwrap());
                    // This will result in NotConnected, anyway ignores that
                    stream.shutdown().await.unwrap_or_default();
                }
                // "rethrow"
                _ => Err(e).unwrap()
            }
        }
    };
}

fn get_state<T>(addr: SocketAddr, selector: impl FnOnce(&ConnectionState) -> T) -> T {
    selector(CONNECTION_POOL.lock().unwrap().get(&addr).unwrap())
}

fn update_state(addr: SocketAddr, update: impl FnOnce(&mut ConnectionState)) {
    update(CONNECTION_POOL.lock().unwrap().get_mut(&addr).unwrap());
}

#[allow(clippy::too_many_lines, clippy::future_not_send, clippy::module_name_repetitions)]
pub async fn telnet_server_service(stream: TcpStream) -> Result<()> {
    let stream = Arc::new(Mutex::new(stream));
    let get_stream = || {
        debug!("getting stream lock");
        let r = stream.lock().unwrap();
        debug!("ok");
        r
    };
    let unknown_command = || async {
        writeln_text_to_stream(&mut get_stream(), "Unknown command. Please type HELP to display help.").await;
    };

    let addr = get_stream().peer_addr().context("get peer addr")?;
    debug!("welcome, {}", &addr);
    {
        // don't mutex lock live long
        CONNECTION_POOL.lock().unwrap().insert(addr, ConnectionState::default());
    }

    let prompt = || async {
        if CONNECTION_POOL.lock().unwrap().get(&addr).unwrap().prompt {
            write_text_to_stream(&mut get_stream(), "toy-blog telnet> ").await;
        }
    };

    let process_command = |parts: Vec<String>| {
        debug!("process command");

        async move {
            let stream = &mut get_stream();
            match parts.len() {
                0 => {
                },
                1 => {
                    let command = parts[0].as_str();
                    match command {
                        "HELP" => {
                            writeln_text_to_stream(stream, "TODO: show well documented help").await;
                        }
                        "MOTD" => {
                            // FIXME: bug?
                            writeln_text_to_stream(stream, "\r\
                                            Please do not send 0x83 via telnet(1).\r\
                                            nc(1) is not affected by this.\r\
                                            NOTE: To see help, please type HELP to prompt.").await;
                        }
                        "DISCONNECT" => {
                            stream.shutdown().await.unwrap();
                        }
                        "LIST" => {
                            match GLOBAL_FILE.parse_file_as_json() {
                                Ok(json) => {
                                    writeln_text_to_stream(stream, "|  ARTICLE ID  | CREATE  DATE | LAST  UPDATE |             CONTENT             |").await;
                                    let x = ListOperationScheme::from(json);
                                    for entry in x.0 {
                                        let content = {
                                            let content = format!(
                                                "{}[END]",
                                                // 改行はテーブルを崩す
                                                entry.entity.content
                                                    .replace('\n', r"\n")
                                                    .replace('\r', r"\r")
                                                    .replace('\t', r"\t")
                                            );
                                            if content.len() >= 30 {
                                                format!("{}...", &content[0..30])
                                            } else {
                                                format!("{content:<33}")
                                            }
                                        };
                                        let article_id = {
                                            let article_id = entry.id.0;
                                            if article_id.len() >= 11 {
                                                format!("{}...", &article_id[0..11])
                                            } else {
                                                format!("{article_id:<14}")
                                            }
                                        };
                                        let line_to_send = format!(
                                            "|{article_id}|  {created_at}  |  {updated_at}  |{content}|",
                                            created_at = entry.entity.created_at.format("%Y-%m-%d"),
                                            updated_at = entry.entity.updated_at.format("%Y-%m-%d"),
                                        );

                                        writeln_text_to_stream(stream, line_to_send.as_str()).await;
                                    }
                                }
                                Err(err) => {
                                    writeln_text_to_stream(stream, format!("Could not get list: {err}").as_str()).await;
                                }
                            };
                        }
                        _ => {
                            unknown_command().await;
                        }
                    }
                },
                2 => {
                    let (command, params) = (parts[0].as_str(), parts[1].as_str());
                    match command {
                        "VAR" => {
                            let vec = params.splitn(2, ' ').collect::<Vec<_>>();
                            let (name, value_opt) = (vec.get(0), vec.get(1));
                            if let Some(name) = name.copied() {
                                match name {
                                    "INTERACTIVE" => {
                                        if let Some(value) = value_opt {
                                            if let Ok(state) = value.to_lowercase().parse() {
                                                update_state(addr, |a| {
                                                    a.prompt = state;
                                                });
                                            } else {
                                                writeln_text_to_stream(stream, "true or false is expected").await;
                                            }
                                        } else {
                                            writeln_text_to_stream(stream, get_state(addr, |f| f.prompt.to_string()).as_str()).await;
                                        }
                                    }
                                    _ => {
                                        writeln_text_to_stream(stream, "unknown variable").await;
                                    }
                                }
                            } else {
                                writeln_text_to_stream(stream, format!("INTERACTIVE={}", get_state(addr, |a| a.prompt)).as_str()).await;
                            }
                        }
                        _ => {
                            unknown_command().await;
                        }
                    }
                },
                _ => unreachable!()
            }
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
