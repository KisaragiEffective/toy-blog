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

    let content = {
        let mut fd = BufReader::new(File::open(file_path)?);
        let mut buf = vec![];
        fd.read_to_end(&mut buf)?;
        String::from_utf8(buf)
    };

    match content {
        Ok(content) => {
            GLOBAL_ARTICLE_REPOSITORY.get().expect("must be fully-initialized").create_entry(article_id, content, Visibility::Private)?;
            info!("Successfully imported as {article_id}.");
            Ok(())
        }
        Err(err) => {
            bail!("The file is not UTF-8: {err}\
                    Please review following list:\
                    - The file is not binary\
                    - The text is encoded with UTF-8\
                    Especially, importing Shift-JIS texts are NOT supported.")
        }
    }
}