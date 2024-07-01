use std::collections::HashMap;
use once_cell::sync::OnceCell;
use crate::service::persistence::ArticleRepository;

// FIXME: OnceCell
pub static GLOBAL_ARTICLE_REPOSITORY: OnceCell<ArticleRepository> = OnceCell::new();
