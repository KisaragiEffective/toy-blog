//! data migration

use serde_json::{Map, Value};
use toy_blog_endpoint_model::Article;

trait SerdeJsonValueMoveExtension {
    ///
    /// # Example
    ///
    /// ```rust
    /// use serde_json::{Map, Value};
    ///
    /// # fn main() {
    /// let mut m = Map::new();
    /// m.insert("a".to_string(), Value::Null);
    /// m.insert("b".to_string(), Value::String("hi".to_string()));
    ///
    /// assert_eq!(Value::Object(m.clone()).into_object(), Ok(m));
    /// assert_eq!(Value::Number(42).into_object(), Err(Value::Number(42)));
    /// # }
    /// ```
    fn into_object(self) -> Result<Map<String, Value>, Value>;
}

impl SerdeJsonValueMoveExtension for Value {
    fn into_object(self) -> Result<Map<String, Value>, Value> {
        match self {
            Value::Object(map) => Ok(map),
            other => Err(other),
        }
    }
}

trait ArticleMigration {
    fn migrate(&self, raw_config: Value) -> Value;
}

struct AddTagVersion;

impl ArticleMigration for AddTagVersion {
    fn migrate(&self, raw_config: Value) -> Value {
        let mut m = raw_config.into_object().expect("top level must be an object");
        m.insert("version".to_string(), Value::from("1"));

        Value::from(m)
    }
}

macro_rules! name_of {
    ($t:tt::$field:ident) => {
        {
            // #[allow(unused)]
            let _ = |arg: &$t| arg.$field;
            stringify!($field)
        }
    };
}

struct AddAccessLevel;

impl ArticleMigration for AddAccessLevel {
    fn migrate(&self, raw_config: Value) -> Value {
        let mut top = raw_config
            .into_object().expect("top level must be an object");

        if top["version"].as_str().expect("version must be string").parse::<i32>().expect("schema version must be fit in i32") >= 2 {
            return Value::from(top)
        }

        top.insert("version".to_string(), Value::from(2));

        let mut article_table = top
            .get_mut("data").expect("article must exist")
            .as_object_mut().expect("article table must be object");

        article_table.values_mut().for_each(|article| {
            article.as_object_mut().expect("article").insert(name_of!(Article::visibility).to_string(), Value::from("public"));
        });

        Value::from(top)
    }
}

pub(crate) fn migrate_article_repr(raw_article_table: Value) -> Value {
    let raw_article_table = AddTagVersion.migrate(raw_article_table);

    AddAccessLevel.migrate(raw_article_table)
}
