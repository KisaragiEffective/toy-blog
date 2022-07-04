use std::fmt::{Display, Formatter};
use serde::{Serialize, Deserialize};

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
