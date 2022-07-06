use std::io::ErrorKind;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[allow(unused_variables)] // false-positive on IntelliJ
#[allow(clippy::module_name_repetitions)]
pub async fn writeln_text_to_stream<'a, 'b: 'a>(stream: &mut TcpStream, text: &'b str) {
    write_text_to_stream(stream, format!("{text}\r\n").as_str()).await;
}

#[allow(clippy::module_name_repetitions)]
pub async fn write_text_to_stream<'a, 'b: 'a>(stream: &'a mut TcpStream, text: &'b str) {
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
