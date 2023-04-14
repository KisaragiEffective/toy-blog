use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use crate::service::telnet::state::TemporaryStatus;

#[derive(Default)]
#[allow(clippy::module_name_repetitions)]
pub struct ConnectionStatusRepository {
    inner: Arc<Mutex<HashMap<SocketAddr, TemporaryStatus>>>,
}

impl ConnectionStatusRepository {
    pub fn get(&self, key: SocketAddr) -> Option<Box<TemporaryStatus>> {
        self.inner.lock().unwrap().get(&key).copied().map(Box::new)
    }

    pub fn get_and_pick<T>(&self, key: SocketAddr, f: impl FnOnce(Box<TemporaryStatus>) -> T) -> Option<T> {
        self.get(key).map(f)
    }

    pub fn update_with(&self, key: SocketAddr, f: impl FnOnce(Box<TemporaryStatus>)) {
        self.get(key).map(f);
    }

    pub fn add(&self, key: SocketAddr, value: TemporaryStatus) {
        self.inner.lock().unwrap().insert(key, value);
    }
}
