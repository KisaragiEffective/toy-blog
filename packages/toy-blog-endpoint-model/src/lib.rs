#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::string::FromUtf8Error;
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleId(pub String);

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

impl FromStr for ArticleId {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

pub type CreateArticleResult = Result<ArticleCreatedNotice, CreateArticleError>;

pub struct ArticleCreatedNotice {
    pub warnings: HashSet<ArticleCreateWarning>,
    pub allocated_id: ArticleId,
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub enum ArticleCreateWarning {
    CurlSpecificNoNewLine,
}

impl Display for ArticleCreateWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ArticleCreateWarning::CurlSpecificNoNewLine =>
                r#"There's no newlines. Perhaps you should use --data-binary instead?
Note: `man curl(1)` said:
    > When -d, --data is told to read from a file like that, carriage
    > returns and newlines  will  be stripped out."#
        };

        f.write_str(s)
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum CreateArticleError {
    Unauthorized,
    DuplicatedArticleId,
    InvalidUtf8,
}

pub type GetArticleResult = Result<OwnedMetadata<ArticleSnapshotMetadata, ArticleSnapshot>, GetArticleError>;

pub struct OwnedMetadata<M, D> {
    pub metadata: M,
    pub data: D
}

pub struct ArticleSnapshotMetadata {
    pub updated_at: chrono::DateTime<FixedOffset>,
}

pub struct ArticleSnapshot {
    pub content: ArticleContent,
}

pub struct ArticleContent(String);

impl ArticleContent {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

pub enum GetArticleError {
    NoSuchArticleFoundById,
}

pub type UpdateArticleResult = Result<(), UpdateArticleError>;

pub enum UpdateArticleError {
    InvalidBearerToken,
    ArticleNotFoundById,
    InvalidByteSequenceForUtf8(FromUtf8Error),
}

pub type DeleteArticleResult = Result<(), DeleteArticleError>;

pub enum DeleteArticleError {
    InvalidBearerToken,
    NoSuchArticleFoundById,
}

pub type ListArticleResult = Result<ListArticleResponse, Infallible>;

#[derive(EnumString, Deserialize, Copy, Clone, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ListArticleSortPolicy {
    Newest,
    Oldest,
    RecentUpdated,
    LeastRecentlyUpdated,
}

#[derive(Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct ListArticleRequestQuery {
    #[serde(rename = "sort")]
    pub policy: Option<ListArticleSortPolicy>,
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct ListArticleResponse(pub Vec<FlatId<ArticleId, Article>>);

#[derive(Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct FlatId<Id, Entity> {
    pub id: Id,
    #[serde(flatten)]
    pub entity: Entity,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ChangeArticleIdRequestQuery {
    pub from: ArticleId,
    pub to: ArticleId,
}

pub type ChangeArticleIdRequestResult = Result<(), ChangeArticleIdError>;

pub enum ChangeArticleIdError {
    Unauthorized,
    ArticleNotFoundById,
}

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Debug)]
pub struct Article {
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub content: String,
}
