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

#[derive(Debug, Clone)]
pub struct ArticleRepository {
    cache: Arc<RwLock<FileScheme>>,
    invalidated: Arc<AtomicBool>,
    file_lock: Arc<RwLock<NamedLockedFile>>,
}

impl ArticleRepository {
    // TODO: visible for test
    pub(super) fn create_default_file_if_absent(path: impl AsRef<Path>) {
        if !path.as_ref().exists() {
            Self::init(path);
        }
    }
    
    pub(super) fn init(path: impl AsRef<Path>) {
        info!("creating article table");
        let mut file = File::options().write(true).read(true).create(true).truncate(false).open(path.as_ref()).unwrap();
        write!(
            &mut (file),
            "{default_json}",
            default_json = serde_json::to_string(&FileScheme::empty()).unwrap()
        ).unwrap();
    }

    pub async fn new(path: impl AsRef<Path>) -> Self {
        Self::create_default_file_if_absent(path.as_ref());

        let mut lock = NamedLockedFile::try_new(
            path.as_ref().to_path_buf(),
            Duration::new(10, 0)
        ).await.expect("failed to lock file");

        Self {
            cache: Arc::new(RwLock::new(Self::parse_file_as_json_static(&mut lock).expect("crash"))),
            invalidated: Arc::new(AtomicBool::new(false)),
            file_lock: Arc::new(RwLock::new(lock)),
        }
    }

    fn invalidate(&self) {
        self.invalidated.store(false, Ordering::SeqCst);
    }

    fn reconstruct_cache(&self) {
        if self.invalidated.load(Ordering::SeqCst) {
            *self.cache.write().expect("cache lock is poisoned") =
                Self::parse_file_as_json_static(&mut self.file_lock.write().expect("poisoned")).expect("crash");
            self.invalidated.store(false, Ordering::SeqCst);
        }
    }

    fn save(&self) -> Result<(), serde_json::Error> {
        let r = &mut **self.file_lock.write().expect("file lock is poisoned");
        r.rewind().expect("seek");
        serde_json::to_writer(
            r,
            &&*self.cache.read().expect("cache is poisoned")
        )?;
        debug!("saved");

        Ok(())
    }

    pub fn create_entry(&self, article_id: &ArticleId, article_content: String, visibility: Visibility) -> Result<(), PersistenceError> {
        self.invalidate();

        let current_date = Local::now();
        self.cache.write().expect("lock is poisoned").data.insert(article_id.clone(), Article {
                created_at: current_date,
                updated_at: current_date,
                // visible: false,
                content: article_content,
                visibility,
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

    pub fn update_entry(&self, article_id: &ArticleId, article_content: String) -> Result<(), PersistenceError> {
        self.invalidate();
        // the lint suggestion causes borrow-to-temporary error
        #[allow(clippy::significant_drop_tightening)]
        let mut m = self.cache.write().expect("cache is poisoned");
        #[allow(clippy::significant_drop_tightening)]
        let find = m.data.get_mut(article_id);
        let Some(article) = find else {
            return Err(PersistenceError::AbsentValue)
        };

        let current_date = Local::now();
        article.updated_at = current_date;
        article.content = article_content;

        self.save()?;
        Ok(())
    }

    // TODO: there's bug that the engine cannot change its visibility.
    pub fn change_visibility(&self, article_id: &ArticleId, new_visibility: Visibility) -> Result<(), PersistenceError> {
        info!("calling change_visibility");
        self.invalidate();

        self.cache.write().expect("poisoned").deref_mut().data.get_mut(article_id)
            .ok_or(PersistenceError::AbsentValue)?.visibility = new_visibility;

        self.save()?;

        Ok(())
    }

    pub fn read_snapshot(&self, article_id: &ArticleId) -> Result<Article, PersistenceError> {
        self.reconstruct_cache();

        let article = self.cache.read().expect("cache is poisoned").deref().data
            .get(article_id)
            .cloned()
            .ok_or(PersistenceError::AbsentValue)?;

        Ok(article)
    }

    pub fn exists(&self, article_id: &ArticleId) -> bool {
        self.reconstruct_cache();

        self.cache.read().expect("cache is poisoned").deref().data.contains_key(article_id)
    }

    pub fn remove(&self, article_id: &ArticleId) -> Result<(), PersistenceError> {
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

            let Some(old_article) = x else {
                return Err(PersistenceError::AbsentValue);
            };

            exclusive_dummy_atomic_guard.data.insert(new_id, old_article);
            drop(exclusive_dummy_atomic_guard);
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

#[cfg(test)]
mod tests {
    use fern::colors::ColoredLevelConfig;
    use toy_blog_endpoint_model::{ArticleId, Visibility};
    use crate::service::persistence::ArticleRepository;

    fn setup_logger() -> anyhow::Result<()> {
        let colors = ColoredLevelConfig::new();
        fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    colors.color(record.level()),
                    message
                ));
            })
            .level(log::LevelFilter::Debug)
            .chain(std::io::stdout())
            // .chain(fern::log_file("output.log")?)
            .apply()?;
        Ok(())
    }

    #[test]
    fn check_file_cursor_position_is_rewinded_to_its_start() {
        /* 想定シナリオ
         * 1. 新しく作成
         * 2. 何らかの記事を作る
         * 3. repoのセッションを閉じる
         * 4. もう一度開こうとした時にエラーにならないこと
         */
        setup_logger().expect("log");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let m = tempfile::NamedTempFile::new().expect("failed to initialize temporary file");
                ArticleRepository::init(m.path());
                let temp_repo = ArticleRepository::new(m.path()).await;
                temp_repo.create_entry(&ArticleId::new("12345".to_string()), "12345 Hello".to_string(), Visibility::Private).expect("failed to save");
                temp_repo.create_entry(&ArticleId::new("23456".to_string()), "23456 Hello".to_string(), Visibility::Private).expect("failed to save");
                drop(temp_repo);
                let temp_repo = ArticleRepository::new(m.path()).await;
                let y = temp_repo.entries().iter().find(|x| x.0 == ArticleId::new("12345".to_string())).expect("12345").1.content == "12345 Hello";
                assert!(y);
                let y = temp_repo.entries().iter().find(|x| x.0 == ArticleId::new("23456".to_string())).expect("23456").1.content == "23456 Hello";
                assert!(y);
            });
    }
}
