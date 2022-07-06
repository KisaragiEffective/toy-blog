use std::net::{IpAddr, SocketAddr};
use log::debug;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::{GLOBAL_FILE, ListOperationScheme};
use crate::telnet::ansi::{ansi_foreground_full_colored, ansi_reset_sequence, bar_color};
use crate::telnet::response::unknown_command;
use crate::telnet::state::{get_state, update_state};
use crate::telnet::stream::writeln_text_to_stream;

#[allow(clippy::too_many_lines)]
pub async fn process_command(parts: Vec<String>, addr: SocketAddr, stream: &mut TcpStream) {
    debug!("handle parts");
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
                    writeln_text_to_stream(stream, [
                        "Please do not send 0x83 via telnet(1).",
                        "nc(1) is not affected by that.",
                        "NOTE: To obtain help, please type HELP.",
                    ]).await;
                }
                "DISCONNECT" => {
                    stream.shutdown().await.unwrap();
                }
                "LIST" => {
                    match GLOBAL_FILE.parse_file_as_json() {
                        Ok(json) => {
                            let separator_color = &bar_color();
                            let body_separator = format!(
                                "{color}+--------------+--------------+--------------+---------------------------------+{reset}",
                                color = ansi_foreground_full_colored(separator_color),
                                reset = ansi_reset_sequence(),
                            );
                            let pipe = &{
                                if get_state(addr, |a| a.colored) {
                                    format!(
                                        "{color}{string}{reset}",
                                        color = ansi_foreground_full_colored(separator_color),
                                        string = '|',
                                        reset = ansi_reset_sequence()
                                    )
                                } else {
                                    "|".to_string()
                                }
                            };
                            writeln_text_to_stream(stream, body_separator.as_str()).await;
                            writeln_text_to_stream(
                                stream,
                                format!("{pipe}  ARTICLE ID  {pipe} CREATE  DATE {pipe} LAST  UPDATE {pipe}             CONTENT             {pipe}").as_str()
                            ).await;
                            writeln_text_to_stream(stream, body_separator.as_str()).await;
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
                                    "{pipe}{article_id}{pipe}  {created_at}  {pipe}  {updated_at}  {pipe}{content}{pipe}",
                                    created_at = entry.entity.created_at.format("%Y-%m-%d"),
                                    updated_at = entry.entity.updated_at.format("%Y-%m-%d"),
                                );

                                writeln_text_to_stream(stream, line_to_send.as_str()).await;
                            }
                            writeln_text_to_stream(stream, body_separator.as_str()).await;
                        }
                        Err(err) => {
                            writeln_text_to_stream(stream, format!("Could not get list: {err}").as_str()).await;
                        }
                    };
                }
                _ => {
                    unknown_command(stream).await;
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
                                        writeln_text_to_stream(stream, "TRUE or FALSE is expected").await;
                                    }
                                } else {
                                    writeln_text_to_stream(stream, get_state(addr, |f| f.prompt.to_string()).as_str()).await;
                                }
                            }
                            "COLOR" => {
                                if let Some(value) = value_opt {
                                    if let Ok(state) = value.to_lowercase().parse() {
                                        update_state(addr, |a| {
                                            a.colored = state;
                                        });
                                    } else {
                                        writeln_text_to_stream(stream, "TRUE or FALSE is expected").await;
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
                        writeln_text_to_stream(stream, format!("COLORED={}", get_state(addr, |a| a.colored)).as_str()).await;
                    }
                }
                _ => {
                    unknown_command(stream).await;
                }
            }
        },
        _ => unreachable!()
    }
}