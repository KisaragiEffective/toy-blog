use tokio::net::TcpStream;
use crate::service::telnet::stream::writeln_text_to_stream;

pub async fn unknown_command(stream: &mut TcpStream) {
    writeln_text_to_stream(stream, "Unknown command. Please type HELP to display help.").await;
}
