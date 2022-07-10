use std::fmt::Display;
use std::net::SocketAddr;
use log::debug;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::{ArticleId, GLOBAL_FILE, ListOperationScheme};
use crate::backend::persistence::Article;
use crate::telnet::ansi::{bar_color, generate_temporary_foreground_color, ToAnsiForegroundColorSequence};
use crate::telnet::response::unknown_command;
use crate::telnet::state::{get_state, update_state};
use crate::telnet::stream::writeln_text_to_stream;

fn switch_color(addr: SocketAddr, base: impl Display, color: impl ToAnsiForegroundColorSequence) -> String {
    if get_state(addr, |a| a.colored) {
        generate_temporary_foreground_color(&color, base)
    } else {
        base.to_string()
    }
}

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
                            let should_colorize = get_state(addr, |a| a.colored);
                            let body_separator = &{
                                let sep = "+--------------+--------------+--------------+---------------------------------+";
                                switch_color(addr, sep, separator_color)
                            };
                            let pipe = switch_color(addr, '|', separator_color);
                            writeln_text_to_stream(stream, body_separator).await;
                            writeln_text_to_stream(
                                stream,
                                format!("{pipe}  ARTICLE ID  {pipe} CREATE  DATE {pipe} LAST  UPDATE {pipe}             CONTENT             {pipe}").as_str()
                            ).await;
                            writeln_text_to_stream(stream, body_separator).await;
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

                                writeln_text_to_stream(stream, line_to_send).await;
                            }
                            writeln_text_to_stream(stream, body_separator).await;
                        }
                        Err(err) => {
                            writeln_text_to_stream(stream, format!("Could not get list: {err}")).await;
                        }
                    };
                }
                "VAR" => {
                    writeln_text_to_stream(stream, format!("PROMPT={}", get_state(addr, |a| a.prompt))).await;
                    writeln_text_to_stream(stream, format!("COLORED={}", get_state(addr, |a| a.colored))).await;
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
                    let (name, value_opt) = (vec[0], vec.get(1));
                    match name {
                        "PROMPT" => {
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
                                writeln_text_to_stream(stream, get_state(addr, |f| f.colored.to_string()).as_str()).await;
                            }
                        }
                        _ => {
                            writeln_text_to_stream(stream, "unknown variable").await;
                        }
                    }
                }
                "GET" => {
                    let id = ArticleId(params.to_string());
                    match GLOBAL_FILE.exists(&id).await {
                        Ok(exists) => {
                            if exists {
                                let article = GLOBAL_FILE.read_snapshot(&id).await;
                                if let Ok(article) = article {
                                    let Article { created_at, updated_at, content } = article;
                                    writeln_text_to_stream(stream, "article found:").await;
                                    writeln_text_to_stream(stream, format!("created at {created_at}")).await;
                                    writeln_text_to_stream(stream, format!("updated at {updated_at}")).await;
                                    writeln_text_to_stream(stream, format!("content:\r\n{content}")).await;
                                } else {
                                    writeln_text_to_stream(stream, "could not fetch specified article").await;
                                }
                            } else {
                                writeln_text_to_stream(stream, "article cloud not be found").await;
                            }
                        }
                        Err(e) => {
                            writeln_text_to_stream(stream, format!("Internal exception: {e}")).await;
                        }
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
