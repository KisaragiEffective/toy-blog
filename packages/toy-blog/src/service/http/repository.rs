use std::fs::File;
use log::info;
use once_cell::sync::OnceCell;
use serde_json::Value;
use crate::service::persistence::ArticleRepository;

// FIXME: OnceCell
pub static GLOBAL_FILE: OnceCell<ArticleRepository> = OnceCell::new();
