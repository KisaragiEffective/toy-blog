use std::fs::File;
use std::path::PathBuf;

use actix_cors::Cors;
use actix_web::http::header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE};
use addr::domain::Name;
use addr::parse_domain_name;
use once_cell::sync::Lazy;
use anyhow::Result;
use serde::{Serialize, Deserialize, Deserializer, Serializer};
use serde_json::de::IoRead;

static CORS_ALLOWED_DOMAINS: Lazy<ParsedAllowOrigin<'_>> = Lazy::new(|| load_allowed_origins().unwrap());

#[derive(Debug, Eq, PartialEq, Default)]
struct ParsedAllowOrigin<'a>(Vec<Name<'a>>);

impl<'de> Deserialize<'de> for ParsedAllowOrigin<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(Self(Vec::deserialize(deserializer)?))
    }
}

impl Serialize for ParsedAllowOrigin<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.serialize(serializer)
    }
}
fn load_allowed_origins<'a>() -> Result<ParsedAllowOrigin<'a>> {
    let path = "data/cors_setting.json";
    if !PathBuf::from(path).exists() {
        File::create(path)?;
        let write = File::options().write(true).open(path)?;
        serde_json::to_writer(write, &ParsedAllowOrigin::default())?;

        return Ok(ParsedAllowOrigin::default())
    }

    // scoped deserialize
    let mut de = serde_json::de::Deserializer::new(IoRead::new(File::open(path)?));
    let value = ParsedAllowOrigin::deserialize(&mut de)?;

    // Make sure the whole stream has been consumed.
    de.end()?;
    Ok(value)
}

pub fn middleware_factory() -> Cors {
    Cors::default()
        .allowed_origin_fn(|origin_value, _| {
            CORS_ALLOWED_DOMAINS.0.contains(&parse_domain_name(origin_value.to_str().unwrap()).unwrap())
        })
        .allowed_headers([CONTENT_TYPE, AUTHORIZATION, ACCEPT, ACCEPT_LANGUAGE])
        .allowed_methods(["GET", "HEAD"])
        .max_age(3600)
}
