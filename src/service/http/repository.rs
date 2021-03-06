use once_cell::sync::Lazy;
use crate::service::persistence::ArticleRepository;

pub static GLOBAL_FILE: Lazy<ArticleRepository> = Lazy::new(|| ArticleRepository::new("data/article.json"));
