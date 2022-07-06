use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

pub static CONNECTION_POOL: Lazy<Arc<Mutex<HashMap<SocketAddr, ConnectionState>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Default)]
pub struct ConnectionState {
    pub prompt: bool,
    pub colored: bool,
}

pub fn get_state<T>(addr: SocketAddr, selector: impl FnOnce(&ConnectionState) -> T) -> T {
    selector(CONNECTION_POOL.lock().unwrap().get(&addr).unwrap())
}

pub fn update_state(addr: SocketAddr, update: impl FnOnce(&mut ConnectionState)) {
    update(CONNECTION_POOL.lock().unwrap().get_mut(&addr).unwrap());
}
