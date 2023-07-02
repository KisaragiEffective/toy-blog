use once_cell::sync::OnceCell;
use crate::service::persistence::ArticleRepository;

pub static GLOBAL_FILE: OnceCell<ArticleRepository> = OnceCell::new();
