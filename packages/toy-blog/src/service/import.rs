use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use anyhow::bail;
use log::{debug, info};
use toy_blog_endpoint_model::{ArticleId, Visibility};
use crate::service::rest::repository::GLOBAL_ARTICLE_REPOSITORY;

pub fn import(file_path: &Path, article_id: &ArticleId) -> Result<(), anyhow::Error> {
    if !file_path.exists() {
        bail!("You can not import non-existent file")
    }

    if !file_path.is_file() {
        // TODO: /dev/stdin is not supported by this method
        debug!("is_dir: {}", file_path.is_dir());
        debug!("is_symlink: {}", file_path.is_symlink());
        debug!("metadata: {:?}", file_path.metadata()?);
        bail!("Non-file paths are not supported")
    }

    let content = std::fs::read_to_string(file_path);

    match content {
        Ok(content) => {
            GLOBAL_ARTICLE_REPOSITORY.get().expect("must be fully-initialized").create_entry(article_id, content, Visibility::Private)?;
            info!("Successfully imported as {article_id}.");
            Ok(())
        }
        Err(err) => {
            bail!("The file could not be read: {err}")
        }
    }
}