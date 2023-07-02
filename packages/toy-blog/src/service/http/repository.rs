use std::fs::File;
use log::info;
use once_cell::sync::OnceCell;
use serde_json::Value;
use crate::service::persistence::ArticleRepository;

// FIXME: OnceCell
pub static GLOBAL_FILE: Lazy<ArticleRepository> = Lazy::new(|| {
    let migrated_data = crate::migration::migrate_article_repr(
        serde_json::from_reader::<_, Value>(File::open("data/article.json").expect("ow, failed!")).expect("ow, failed!")
    );

    info!("migrated");

    serde_json::to_writer(File::options().write(true).truncate(true).open("data/article.json").expect("ow, failed!"), &migrated_data)
        .expect("ow, failed!");
    ArticleRepository::new("data/article.json")
});
