use actix_web::body::{BoxBody, MessageBody};
use actix_web::{HttpResponse, HttpResponseBuilder};

pub trait RespondPlainText<T: MessageBody + 'static> {
    fn respond_with_auto_charset(&mut self, body: T) -> HttpResponse<BoxBody>;
}

impl RespondPlainText<String> for HttpResponseBuilder {
    fn respond_with_auto_charset(&mut self, body: String) -> HttpResponse<BoxBody> {
        use actix_web::http::header::ContentType;

        self
            .insert_header(ContentType::plaintext())
            .body(body)
    }
}

impl RespondPlainText<&'static str> for HttpResponseBuilder {
    fn respond_with_auto_charset(&mut self, body: &'static str) -> HttpResponse<BoxBody> {
        use actix_web::http::header::ContentType;

        self
            .insert_header(ContentType::plaintext())
            .body(body)
    }
}
