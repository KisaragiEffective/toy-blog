use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use chrono::Local;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toy_blog_endpoint_model::{ArticleId, FlatId, ListArticleResponse};

pub struct ArticleRepository {
    path: PathBuf,
    lock: RwLock<()>
}

impl ArticleRepository {
    fn create_default_file_if_absent(path: impl AsRef<Path>) {
        if !path.as_ref().exists() {
            let mut file = File::options().write(true).read(true).create(true).open(path.as_ref()).unwrap();
            write!(
                &mut (file),
                "{default_json}",
                default_json = serde_json::to_string(&FileScheme::empty()).unwrap()
            ).unwrap();
        }
    }

    // TODO: 誤って同じパスに対してこのメソッドを二回以上呼ぶと破滅する
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self::create_default_file_if_absent(path.as_ref());

        Self {
            path: path.as_ref().to_path_buf(),
            lock: RwLock::new(())
        }
    }

    fn get_overwrite_handle(&self) -> (std::io::Result<File>, RwLockWriteGuard<'_, ()>) {
        (File::options().write(true).truncate(true).open(&self.path), self.lock.write().unwrap())
    }

    fn get_read_handle(&self) -> (std::io::Result<File>, RwLockReadGuard<'_, ()>) {
        (File::options().read(true).open(&self.path), self.lock.read().unwrap())
    }

    pub async fn create_entry(&self, article_id: &ArticleId, article_content: String) -> Result<(), PersistenceError> {
        info!("calling add_entry");
        let mut a = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            let current_date = Local::now();
            a.data.insert(article_id.clone(), Article {
                created_at: current_date,
                updated_at: current_date,
                // visible: false,
                content: article_content,
            });
            info!("modified");
        }

        trace!("saving");
        serde_json::to_writer(file, &a)?;
        trace!("saved");
        Ok(())
    }

    pub async fn update_entry(&self, article_id: &ArticleId, article_content: String) -> Result<(), PersistenceError> {
        info!("calling add_entry");
        let mut fs = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            let current_date = Local::now();
            match fs.data.get_mut(article_id) {
                None => {
                    return Err(PersistenceError::AbsentValue)
                }
                Some(article) => {
                    article.updated_at = current_date;
                    article.content = article_content;
                }
            }
            info!("modified");
        }

        serde_json::to_writer(file, &fs)?;
        info!("wrote");
        Ok(())
    }

    pub async fn read_snapshot(&self, article_id: &ArticleId) -> Result<Article, PersistenceError> {
        info!("calling read");
        let a = self.parse_file_as_json()?;
        let q = a.data.get(article_id).cloned();
        q.ok_or(PersistenceError::AbsentValue)
    }

    pub async fn exists(&self, article_id: &ArticleId) -> Result<bool, PersistenceError> {
        info!("calling exists");
        let a = self.parse_file_as_json()?;
        Ok(a.data.contains_key(article_id))
    }

    pub async fn remove(&self, article_id: &ArticleId) -> Result<(), PersistenceError> {
        info!("calling remove");
        let mut a = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            a.data.remove(article_id);
            info!("modified");
        }

        let json = serde_json::to_string(&a)?;
        write!(
            &mut BufWriter::new(&file),
            "{json}"
        )?;

        info!("wrote");
        Ok(())
    }

    pub fn parse_file_as_json(&self) -> Result<FileScheme, PersistenceError> {
        let (file, _lock) = self.get_read_handle();
        let mut read_all = BufReader::new(file?);
        let mut buf = vec![];
        read_all.read_to_end(&mut buf)?;
        let got = String::from_utf8(buf)?;
        debug!("parsed");
        trace!("got: {got}", got = &got);

        let j = serde_json::from_str(got.as_str()).map_err(|e| {
            error!("{e}", e = &e);
            e
        })?;

        Ok(j)
    }

    pub fn rename(&self, old_id: &ArticleId, new_id: ArticleId) -> Result<(), PersistenceError> {
        debug!("rename");
        let mut repo = self.parse_file_as_json()?.data;
        debug!("parsed");
        let (_file, _lock) = self.get_overwrite_handle();

        if repo.get(old_id).is_some() {
            repo.remove(old_id);
            let data = repo.get(old_id).unwrap();
            repo.insert(new_id, data.clone());
            Ok(())
        } else {
            Err(PersistenceError::AbsentValue)
        }
    }
}

#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum PersistenceError {
    #[error("I/O Error: {_0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8: {_0}")]
    Utf8(#[from] FromUtf8Error),
    #[error("JSON deserialize error: {_0}")]
    JsonDeserialize(#[from] serde_json::Error),
    #[error("Absent value")]
    AbsentValue,
}

#[derive(Serialize, Deserialize)]
pub struct FileScheme {
    pub(in crate::service) data: HashMap<ArticleId, Article>
}

impl FileScheme {
    fn empty() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl From<FileScheme> for ListArticleResponse {
    fn from(val: FileScheme) -> Self {
        Self(val.data.into_iter().map(|(id, entity)| FlatId {
            id,
            entity,
        }).collect::<Vec<_>>())
    }
}

use toy_blog_endpoint_model::Article;
