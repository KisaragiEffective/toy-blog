use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use log::{error, info};
use serde::{Serialize, Deserialize};

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

    fn get_overwrite_handle(&self) -> (Result<File>, RwLockWriteGuard<'_, ()>) {
        (File::options().write(true).truncate(true).open(&self.path).context("open file"), self.lock.write().unwrap())
    }

    fn get_read_handle(&self) -> (Result<File>, RwLockReadGuard<'_, ()>) {
        (File::options().read(true).open(&self.path).context("open file"), self.lock.read().unwrap())
    }

    pub async fn create_entry(&self, article_id: &ArticleId, article_content: String) -> Result<()> {
        info!("calling add_entry");
        let mut a = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            let current_date = Local::now();
            (&mut a.data).insert(article_id.clone(), Article {
                created_at: current_date,
                updated_at: current_date,
                // visible: false,
                content: article_content,
            });
            info!("modified");
        }

        serde_json::to_writer(file, &a)?;
        info!("wrote");
        Ok(())
    }

    pub async fn update_entry(&self, article_id: &ArticleId, article_content: String) -> Result<()> {
        info!("calling add_entry");
        let mut fs = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            let current_date = Local::now();
            match (&mut fs.data).get_mut(article_id) {
                None => {
                    bail!("article must be exists")
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

    pub async fn read_snapshot(&self, article_id: &ArticleId) -> Result<Article> {
        info!("calling read");
        let a = self.parse_file_as_json()?;
        a.data.get(article_id).cloned().context(format!("read_snapshot: failed to get {article_id:?}"))
    }

    pub async fn exists(&self, article_id: &ArticleId) -> Result<bool> {
        info!("calling exists");
        let a = self.parse_file_as_json()?;
        Ok(a.data.contains_key(article_id))
    }

    pub async fn remove(&self, article_id: &ArticleId) -> Result<()> {
        info!("calling remove");
        let mut a = self.parse_file_as_json()?;
        info!("parsed");
        let (file, _lock) = self.get_overwrite_handle();
        let file = file?;

        {
            (&mut a.data).remove(article_id);
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

    pub(in crate::backend) fn parse_file_as_json(&self) -> Result<FileScheme> {
        let (file, _lock) = self.get_read_handle();
        let mut read_all = BufReader::new(file?);
        let mut buf = vec![];
        read_all.read_to_end(&mut buf).context("verify file")?;
        let got = String::from_utf8(buf).context("utf8 verify")?;
        info!("file JSON: {got}", got = &got);

        serde_json::from_str(got.as_str()).map_err(|e| {
            error!("{e}", e = &e);
            e
        }).context("reading json file")
    }
}

#[derive(Serialize, Deserialize)]
pub(in crate::backend) struct FileScheme {
    pub(in crate::backend) data: HashMap<ArticleId, Article>
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub(in crate::backend) struct ListOperationScheme(Vec<FlatId<ArticleId, Article>>);

impl From<FileScheme> for ListOperationScheme {
    fn from(fs: FileScheme) -> Self {
        Self(
            fs.data.into_iter().map(|(k, v)| {
                FlatId {
                    id: k,
                    entity: v
                }
            }).collect()
        )
    }
}

#[derive(Eq, PartialEq, Debug, Deserialize, Serialize)]
struct FlatId<Id, Entity> {
    id: Id,
    #[serde(flatten)]
    entity: Entity,
}

impl FileScheme {
    fn empty() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Debug)]
pub struct Article {
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub content: String,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleId(String);

impl ArticleId {
    pub const fn new(s: String) -> Self {
        Self(s)
    }
}

impl Display for ArticleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}