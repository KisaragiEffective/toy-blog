use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use crate::telnet::state::TemporaryStatus;

#[derive(Default)]
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

    pub fn get_mut(&self, key: SocketAddr) -> Option<Box<TemporaryStatus>> {
        self.get(key)
    }

    pub fn get_unchecked(&self, key: SocketAddr) -> Box<TemporaryStatus> {
        self.get(key).unwrap()
    }

    pub fn update_with(&self, key: SocketAddr, f: impl FnOnce(Box<TemporaryStatus>)) {
        self.get_mut(key).map(f);
    }

    pub fn add(&self, key: SocketAddr, value: TemporaryStatus) {
        self.inner.lock().unwrap().insert(key, value);
    }

    pub fn delete(&self, key: SocketAddr) {
        self.inner.lock().unwrap().remove(&key);
    }
}
