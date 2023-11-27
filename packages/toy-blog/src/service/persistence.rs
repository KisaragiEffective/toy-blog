use std::collections::{HashMap};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use chrono::Local;
use fs2::FileExt;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toy_blog_endpoint_model::{ArticleId, FlatId, ListArticleResponse, Visibility};

#[derive(Debug)]
pub struct ArticleRepository {
    cache: Arc<RwLock<FileScheme>>,
    invalidated: AtomicBool,
    file_lock: Arc<RwLock<NamedLockedFile>>,
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

    pub async fn new(path: impl AsRef<Path>) -> Self {
        Self::create_default_file_if_absent(path.as_ref());

        let mut lock = NamedLockedFile::try_new(
            path.as_ref().to_path_buf(),
            Duration::new(10, 0)
        ).await.expect("failed to lock file");

        Self {
            cache: Arc::new(RwLock::new(Self::parse_file_as_json_static(&mut lock).expect("crash"))),
            invalidated: AtomicBool::new(false),
            file_lock: Arc::new(RwLock::new(lock)),
        }
    }

    fn invalidate(&self) {
        self.invalidated.store(false, Ordering::SeqCst);
    }

    fn reconstruct_cache(&self) {
        if self.invalidated.load(Ordering::SeqCst) {
            *self.cache.write().expect("cache lock is poisoned").deref_mut() =
                Self::parse_file_as_json_static(&mut self.file_lock.write().expect("poisoned")).expect("crash");
            self.invalidated.store(false, Ordering::SeqCst);
        }
    }

    fn save(&self) -> Result<(), serde_json::Error> {
        serde_json::to_writer(
            &mut **self.file_lock.write().expect("file lock is poisoned"),
            &&*self.cache.read().expect("cache is poisoned")
        )?;
        trace!("saved");

        Ok(())
    }

    pub async fn create_entry(&self, article_id: &ArticleId, article_content: String, visibility: Visibility) -> Result<(), PersistenceError> {
        self.invalidate();

        let current_date = Local::now();
        self.cache.write().expect("lock is poisoned").data.insert(article_id.clone(), Article {
                created_at: current_date,
                updated_at: current_date,
                // visible: false,
                content: article_content,
                visibility: Some(visibility),
            });


        self.save()?;
        Ok(())
    }

    /// it is not guaranteed that the elements are sorted in particular order.
    pub fn entries(&self) -> Vec<(ArticleId, Article)> {
        self.reconstruct_cache();

        self.cache.read().expect("cache is poisoned").deref().data
            .iter()
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect()
    }

    pub async fn update_entry(&self, article_id: &ArticleId, article_content: String) -> Result<(), PersistenceError> {
        self.invalidate();

        match self.cache.write().expect("cache is poisoned").data.get_mut(article_id) {
            None => {
                return Err(PersistenceError::AbsentValue)
            }
            Some(article) => {
                let current_date = Local::now();
                article.updated_at = current_date;
                article.content = article_content;
            }
        }

        self.save()?;
        Ok(())
    }

    pub async fn change_visibility(&self, article_id: &ArticleId, new_visibility: Visibility) -> Result<(), PersistenceError> {
        info!("calling change_visibility");
        self.invalidate();

        self.cache.write().expect("poisoned").deref_mut().data.get_mut(article_id)
            .ok_or(PersistenceError::AbsentValue)?.visibility = Some(new_visibility);

        self.save()?;

        Ok(())
    }

    pub async fn read_snapshot(&self, article_id: &ArticleId) -> Result<Article, PersistenceError> {
        self.reconstruct_cache();

        let article = self.cache.read().expect("cache is poisoned").deref().data
            .get(article_id)
            .cloned()
            .ok_or(PersistenceError::AbsentValue)?;

        Ok(article)
    }

    pub async fn exists(&self, article_id: &ArticleId) -> Result<bool, PersistenceError> {
        self.reconstruct_cache();

        let contains = self.cache.read().expect("cache is poisoned").deref().data.contains_key(article_id);

        Ok(contains)
    }

    pub async fn remove(&self, article_id: &ArticleId) -> Result<(), PersistenceError> {
        info!("calling remove");

        self.invalidate();

        self.cache.write().expect("cache is poisoned").deref_mut().data.remove(article_id);

        self.save()?;

        Ok(())
    }

    fn parse_file_as_json_static(locked: &mut NamedLockedFile) -> Result<FileScheme, PersistenceError> {
        locked.file.seek(SeekFrom::Start(0)).expect(".");

        let mut read_all = BufReader::new(&mut locked.file);
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
        self.invalidate();
        #[allow(clippy::significant_drop_tightening)]
        let not_found_old_id = !self.cache.read().expect("cache is poisoned").deref().data.contains_key(&new_id);

        let mut exclusive_dummy_atomic_guard = self.cache.write().expect("cache is poisoned");

        if not_found_old_id {
            let x = exclusive_dummy_atomic_guard.data.remove(old_id);

            if let Some(old_article) = x {
                exclusive_dummy_atomic_guard.data.insert(new_id, old_article);
            } else {
                return Err(PersistenceError::AbsentValue)
            }
        }

        self.save()?;

        Ok(())
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Debug)]
struct NamedLockedFile {
    file: File,
    path: PathBuf,
}

impl NamedLockedFile {
    async fn try_new(path: PathBuf, timeout: Duration) -> Result<Self, FileLockError> {
        tokio::time::timeout(timeout, async {
            let f = File::options().read(true).write(true).open(&path)?;
            f.lock_exclusive()?;

            Ok(Self {
                file: f,
                path,
            })
        }).into_inner().await
    }
}

impl Drop for NamedLockedFile {
    fn drop(&mut self) {
        if let Some(x) = self.file.unlock().err() {
            error!("unable to unlock article entry, ignoring error. detail: {x:?}");
        }
    }
}

impl Deref for NamedLockedFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for NamedLockedFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

#[derive(Debug, Error)]
enum FileLockError {
    #[error("IO: {0:?}")]
    Io(#[from] std::io::Error),
    #[error("timeout")]
    Timeout,
}
