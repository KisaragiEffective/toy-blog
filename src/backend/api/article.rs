use actix_web::Responder;
use actix_web::{get, post, delete};
use actix_web::web::Path;

#[post("/create")]
pub async fn create() -> impl Responder {
    todo!();
    "todo"
}

#[get("/get/{id}")]
pub async fn fetch(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}

#[get("/list")]
pub async fn list() -> impl Responder {
    todo!();
    "todo"
}

#[post("/update/{article_id}")]
pub async fn update(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}

#[delete("/delete/{article_id}")]
pub async fn remove(path: Path<i32>) -> impl Responder {
    todo!();
    "todo"
}
