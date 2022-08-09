use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use once_cell::sync::OnceCell;
use crate::extension::RespondPlainText;

pub(in super) fn is_wrong_token(token: &str) -> bool {
    let correct_token = WRITE_TOKEN.get().unwrap().as_str();
    correct_token != token
}

pub(in super) fn unauthorized() -> HttpResponse {
    HttpResponse::build(StatusCode::UNAUTHORIZED)
        .respond_with_auto_charset("You must be authorized to perform this action.")
}

pub static WRITE_TOKEN: OnceCell<String> = OnceCell::new();
