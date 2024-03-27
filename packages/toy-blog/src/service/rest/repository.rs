use once_cell::sync::OnceCell;
use crate::service::persistence::ArticleRepository;

// FIXME: OnceCell
pub static GLOBAL_FILE: OnceCell<ArticleRepository> = OnceCell::new();
