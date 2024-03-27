//! data migration

use log::info;
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
            Self::Object(map) => Ok(map),
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

/// フィールドの名前を静的に参照し、その名前をコンパイル時に確定する文字列リテラルへ焼く。
/// 
/// # Example
/// 
/// ```
/// struct X {
///     example: i32
/// }
/// 
/// # fn main() {
/// const Y: &str = name_of!(X::example);
/// assert_eq!(Y, "example");
/// # }
/// ```
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

        #[allow(clippy::cast_possible_truncation)]
        let version = match &top["version"] {
            Value::Number(n) => n.as_u64().expect("schema version must be fit in u64") as i32,
            Value::String(s) => s.parse::<i32>().expect("schema version must be fit in i32"),
            otherwise => panic!("version must be number or string, but got {otherwise}")
        };
        
        
        if version >= 2 {
            return Value::from(top)
        }

        top.insert("version".to_string(), Value::from(2));

        let article_table = top
            .get_mut("data").expect("article must exist")
            .as_object_mut().expect("article table must be object");

        article_table.iter_mut().for_each(|(key, article)| {
            // すでに存在するならマイグレーションをスキップ
            let article = article.as_object_mut().expect("article");
            let field_to_add = name_of!(Article::visibility);
            if !article.contains_key(field_to_add) {
                info!("migration[AddAccess]: visibility of {key} is now public.");
                article.insert(field_to_add.to_string(), Value::from("public"));
            }
        });

        Value::from(top)
    }
}

pub fn migrate_article_repr(raw_article_table: Value) -> Value {
    info!("migration: start");
    let raw_article_table = AddTagVersion.migrate(raw_article_table);

    let last = AddAccessLevel.migrate(raw_article_table);
    info!("migration: finished");
    last
}

#[cfg(test)]
mod tests {
    // TODO: arbitrary data
    use serde_json::json;
    use crate::migration::{AddAccessLevel, ArticleMigration};

    #[test]
    fn aal_early_return() {
        let a = json!({"version": 2, "data": {}});
        let b = AddAccessLevel.migrate(a);
        assert_eq!(b, json!({"version": 2, "data": {}}));
    }

    #[test]
    fn aal_add_visibility() {
        let a = json!({"version": "1", "data": { "a": { "some_random_data_here": 42 } }});
        let b = AddAccessLevel.migrate(a);
        assert_eq!(b, json!({"version": 2, "data": { "a": { "some_random_data_here": 42, "visibility": "public" } }}));
    }
    
    #[test]
    fn aal_do_not_destroy_visibility() {
        let a = json!({"version": 2, "data": { "a": { "some_random_data_here": 42, "visibility": "private" } }});
        let b = AddAccessLevel.migrate(a);
        assert_eq!(b, json!({"version": 2, "data": { "a": { "some_random_data_here": 42, "visibility": "private" } }}));
    }
}
