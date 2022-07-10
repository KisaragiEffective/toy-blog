use std::net::SocketAddr;
use log::{debug, error};
use once_cell::sync::Lazy;
use crate::telnet::repository::ConnectionStatusRepository;

pub static CONNECTION_POOL: Lazy<ConnectionStatusRepository> = Lazy::new(|| ConnectionStatusRepository::default());

#[derive(Default, Copy, Clone)]
pub struct TemporaryStatus {
    pub prompt: bool,
    pub colored: bool,
}
