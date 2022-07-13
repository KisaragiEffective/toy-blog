use std::fs::File;
use std::path::PathBuf;

use actix_cors::Cors;
use actix_web::http::header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::de::IoRead;

static CORS_ALLOWED_DOMAINS: Lazy<ParsedAllowOrigin> =
    Lazy::new(|| load_allowed_origins().unwrap());

type ParsedAllowOrigin = Vec<String>;

fn load_allowed_origins() -> Result<ParsedAllowOrigin> {
    let path = "data/cors_setting.json";
    if !PathBuf::from(path).exists() {
        File::create(path)?;
        let write = File::options().write(true).open(path)?;
        serde_json::to_writer(write, &ParsedAllowOrigin::default())?;

        return Ok(ParsedAllowOrigin::default());
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
            CORS_ALLOWED_DOMAINS
                .iter()
                .any(|s| s.as_str() == origin_value.to_str().unwrap())
        })
        .allowed_headers([CONTENT_TYPE, AUTHORIZATION, ACCEPT, ACCEPT_LANGUAGE])
        .allowed_methods(["GET", "HEAD"])
        .max_age(3600)
}
