use once_cell::sync::Lazy;
use crate::service::telnet::repository::ConnectionStatusRepository;

pub static CONNECTION_POOL: Lazy<ConnectionStatusRepository> = Lazy::new(ConnectionStatusRepository::default);

#[derive(Default, Copy, Clone)]
pub struct TemporaryStatus {
    pub prompt: bool,
    pub colored: bool,
}
