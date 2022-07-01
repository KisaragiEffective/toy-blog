use actix_web::Responder;
use actix_web::{get, post, put, delete};
use actix_web::web::Path;

// TODO: らぎブログフロントエンド作りたいからCORSヘッダー設定してくれ - @yanorei32

#[post("/{article_id}")]
pub async fn create() -> impl Responder {
    todo!();
    "todo"
}

#[get("/{article_id}")]
pub async fn fetch(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}

#[put("/{article_id}")]
pub async fn update(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}

#[delete("/delete/{article_id}")]
pub async fn remove(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}

#[get("/articles")]
pub async fn list() -> impl Responder {
    todo!();
    "todo"
}
