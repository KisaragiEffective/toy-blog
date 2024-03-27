#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::num::{NonZeroU32, NonZeroU8};
use std::str::FromStr;
use std::string::FromUtf8Error;
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::Error;
use strum::EnumString;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleId(pub String);

impl ArticleId {
    #[must_use] pub const fn new(s: String) -> Self {
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
            Self::CurlSpecificNoNewLine =>
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
    #[must_use] pub const fn new(s: String) -> Self {
        Self(s)
    }

    #[allow(clippy::missing_const_for_fn)]
    #[must_use] pub fn into_inner(self) -> String {
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
    pub visibility: Visibility
}

#[derive(Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Restricted,
    Private,
}

#[derive(Serialize, Clone, Eq, PartialEq, Debug)]
pub struct ArticleIdSet(pub HashSet<ArticleId>);

pub struct ArticleIdSetMetadata {
    pub oldest_created_at: Option<DateTime<Local>>,
    pub newest_updated_at: Option<DateTime<Local>>,
}

#[derive(Deserialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct AnnoDominiYear(NonZeroU32);

impl AnnoDominiYear {
    #[must_use] pub const fn into_inner(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for AnnoDominiYear {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        value.try_into().map(Self).map_err(|_| ())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct OneOriginTwoDigitsMonth(NonZeroU8);

impl OneOriginTwoDigitsMonth {
    #[must_use] pub const fn into_inner(self) -> u8 {
        self.0.get()
    }
}

impl TryFrom<u8> for OneOriginTwoDigitsMonth {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (1..=12).contains(&value) {
            // I believe rustc optimize unwind branch away, because its infallible.
            Ok(Self(value.try_into().unwrap()))
        } else {
            Err(())
        }
    }
}

impl FromStr for OneOriginTwoDigitsMonth {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let try_new = Self::try_from;

        match s {
            "01" => try_new(1),
            "02" => try_new(2),
            "03" => try_new(3),
            "04" => try_new(4),
            "05" => try_new(5),
            "06" => try_new(6),
            "07" => try_new(7),
            "08" => try_new(8),
            "09" => try_new(9),
            "10" => try_new(10),
            "11" => try_new(11),
            "12" => try_new(12),
            _ => Err(())
        }
    }
}

impl<'de> Deserialize<'de> for OneOriginTwoDigitsMonth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let x = s.parse().map_err(|_| D::Error::custom("bad value"))?;
        
        Ok(x)
    }
}

#[derive(Deserialize)]
pub struct UpdateVisibilityPayload {
    pub visibility: Visibility,
}
