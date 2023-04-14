use once_cell::sync::OnceCell;

pub(in super) fn is_wrong_token(token: &str) -> bool {
    let correct_token = WRITE_TOKEN.get().unwrap().as_str();
    correct_token != token
}

pub static WRITE_TOKEN: OnceCell<String> = OnceCell::new();
