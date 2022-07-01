use std::sync::{Arc, RwLock};
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::{get, post, put, delete};
use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Path, ReqData};
use log::info;
use once_cell::sync::Lazy;
use crate::backend::persistence::{ArticleId, ArticleRepository};

static GLOBAL_FILE: Lazy<ArticleRepository> = Lazy::new(|| ArticleRepository::new("index.json"));

// TODO: らぎブログフロントエンド作りたいからCORSヘッダー設定してくれ - @yanorei32

#[post("/{article_id}")]
pub async fn create(path: Path<String>, data: Bytes) -> impl Responder {
    let path = ArticleId::new(path.into_inner());
    info!("create");
    if GLOBAL_FILE.exists(&path).await.unwrap() {
        return HttpResponse::build(StatusCode::CONFLICT)
            .body("already exist. Please choose another one, or overwrite with PUT request.")
    }

    let plain_text = String::from_utf8(data.to_vec());
    if let Ok(text) = plain_text {
        info!("valid utf8");
        let res = GLOBAL_FILE.add_entry(path.clone(), text).await;
        let success = res.is_ok();
        if success {
            HttpResponse::build(StatusCode::OK)
                .body(format!("OK, saved as {path}.", path = &path))
        } else {
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Exception: {err}", err = res.err().unwrap()))
        }
    } else {
        info!("invalid utf8");
        HttpResponse::build(StatusCode::BAD_REQUEST)
            .body("text must be valid UTF-8")
    }
}

#[get("/{article_id}")]
pub async fn fetch(path: Path<String>) -> impl Responder {
    todo!();
    "todo"
}

#[put("/{article_id}")]
pub async fn update(path: Path<String>) -> impl Responder {
    todo!();
    "todo"
}

#[delete("/{article_id}")]
pub async fn remove(path: Path<String>) -> impl Responder {
    todo!();
    "todo"
}

#[get("/articles")]
pub async fn list() -> impl Responder {
    todo!();
    "todo"
}
