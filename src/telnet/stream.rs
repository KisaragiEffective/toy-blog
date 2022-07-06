use std::io::ErrorKind;
use log::debug;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub trait TelnetMessageOutput {
    fn as_telnet_output(&self) -> String;
}

impl TelnetMessageOutput for String {
    fn as_telnet_output(&self) -> String {
        self.clone()
    }
}

impl TelnetMessageOutput for str {
    fn as_telnet_output(&self) -> String {
        self.to_string()
    }
}

impl TelnetMessageOutput for &str {
    fn as_telnet_output(&self) -> String {
        (*self).to_string()
    }
}

impl<T: TelnetMessageOutput> TelnetMessageOutput for [T] {
    fn as_telnet_output(&self) -> String {
        self.iter().map(TelnetMessageOutput::as_telnet_output).collect::<Vec<_>>().join("\r\n")
    }
}

impl<T: TelnetMessageOutput, const N: usize> TelnetMessageOutput for [T; N] {
    fn as_telnet_output(&self) -> String {
        (self as &[T]).as_telnet_output()
    }
}

#[allow(unused_variables)] // false-positive on IntelliJ
#[allow(clippy::module_name_repetitions)]
pub async fn writeln_text_to_stream<'a>(stream: &mut TcpStream, message: impl TelnetMessageOutput) {
    write_text_to_stream(stream, format!("{message}\r\n", message = message.as_telnet_output())).await;
}

#[allow(clippy::module_name_repetitions)]
pub async fn write_text_to_stream(stream: &mut TcpStream, text: impl TelnetMessageOutput) {
    match stream.write_all(text.as_telnet_output().as_bytes()).await {
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
