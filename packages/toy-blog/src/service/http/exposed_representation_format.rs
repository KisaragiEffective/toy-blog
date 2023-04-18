#[cfg(test)]
mod tests;

use std::fmt::{Display, Formatter};
use std::iter::{Empty, empty, once, Once};
use actix_web::HttpResponse;
use actix_web::http::header::{CONTENT_TYPE, HeaderName, HeaderValue, LAST_MODIFIED, WARNING};
use actix_web::http::StatusCode;
use chrono::{FixedOffset, Utc};
use serde::{Serialize, Serializer};
use toy_blog_endpoint_model::{ArticleCreatedNotice, ChangeArticleIdError, ChangeArticleIdRequestResult, CreateArticleError, CreateArticleResult, DeleteArticleError, DeleteArticleResult, GetArticleError, GetArticleResult, ArticleIdSet, ArticleIdSetMetadata, ListArticleResponse, ListArticleResult, OwnedMetadata, UpdateArticleError, UpdateArticleResult};
use crate::service::http::inner_no_leak::{ComposeInternalError, UnhandledError};

type Pair = (HeaderName, HeaderValueUpdateMethod);

pub trait IntoPlainText {
    fn into_plain_text(self) -> String;
}

impl<T: IntoPlainText> IntoPlainText for ComposeInternalError<T> {
    fn into_plain_text(self) -> String {
        match self {
            Ok(t) => t.into_plain_text(),
            Err(e) => e.into_plain_text(),
        }
    }
}

impl IntoPlainText for UnhandledError {
    fn into_plain_text(self) -> String {
        format!("Exception: {self}")
    }
}

pub trait ContainsHeaderMap {
    type Iterator: Iterator<Item = Pair>;

    fn response_headers(&self) -> Self::Iterator;
}

pub enum EitherIter<
    IA: Iterator<Item = I>,
    IB: Iterator<Item = I>,
    I,
> {
    Left(IA),
    Right(IB)
}

impl<
    IA: Iterator<Item = I>,
    IB: Iterator<Item = I>,
    I,
> Iterator for EitherIter<IA, IB, I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(i) => i.next(),
            Self::Right(i) => i.next(),
        }
    }
}

impl<C: ContainsHeaderMap> ContainsHeaderMap for ComposeInternalError<C> {
    type Iterator = EitherIter<
        C::Iterator,
        <UnhandledError as ContainsHeaderMap>::Iterator,
        Pair
    >;

    fn response_headers(&self) -> Self::Iterator {
        match self {
            Ok(c) => EitherIter::Left(c.response_headers()),
            Err(e) => EitherIter::Right(e.response_headers())
        }
    }
}

impl ContainsHeaderMap for UnhandledError {
    type Iterator = VecIter<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        vec![].into_iter()
    }
}

#[derive(Eq, PartialEq)]
pub enum HeaderValueUpdateMethod {
    Overwrite(HeaderValue),
    Append(HeaderValue),
}

pub trait HttpStatusCode {
    fn call_status_code(&self) -> StatusCode;
}

impl<K: HttpStatusCode> HttpStatusCode for ComposeInternalError<K> {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(x) => x.call_status_code(),
            Err(e) => e.call_status_code(),
        }
    }
}

impl HttpStatusCode for UnhandledError {
    fn call_status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub struct EndpointRepresentationCompiler<T>(T);

impl<T> EndpointRepresentationCompiler<T> {
    pub const fn from_value(value: T) -> Self {
        Self(value)
    }
}

impl<T: IntoPlainText + HttpStatusCode + ContainsHeaderMap> EndpointRepresentationCompiler<T> {
    pub fn into_plain_text(self) -> HttpResponse<String> {
        let mut res = HttpResponse::new(self.0.call_status_code());
        res.headers_mut().insert(CONTENT_TYPE, "text/plain; charset=utf-8".try_into().unwrap());
        let x = self.0;
        x.response_headers().for_each(|(k, v)| {
            match v {
                HeaderValueUpdateMethod::Overwrite(v) => {
                    res.headers_mut().insert(k, v);
                }
                HeaderValueUpdateMethod::Append(v) => {
                    res.headers_mut().append(k, v);
                }
            }
        });

        res.set_body(x.into_plain_text())
    }
}

impl<T: Serialize + HttpStatusCode + ContainsHeaderMap> EndpointRepresentationCompiler<T> {
    pub fn into_json(self) -> HttpResponse<T> {
        let mut res = HttpResponse::new(self.0.call_status_code());
        res.headers_mut().insert(CONTENT_TYPE, "application/json".try_into().unwrap());
        res.set_body(self.0)
    }
}


pub struct HttpFormattedDate(chrono::DateTime<FixedOffset>);

impl HttpFormattedDate {
    pub const fn new(v: chrono::DateTime<FixedOffset>) -> Self {
        Self(v)
    }
}

impl Display for HttpFormattedDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dt = &self.0;
        let gmt_datetime = dt.with_timezone(&Utc);
        // Last-Modified: <day-name>, <day> <month> <year> <hour>:<minute>:<second> GMT
        f.write_str(&gmt_datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
    }
}

type VecIter<T> = <Vec<T> as IntoIterator>::IntoIter;
// --------------------------

impl HttpStatusCode for CreateArticleResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(_) => StatusCode::CREATED,
            Err(f) => {
                match f {
                    CreateArticleError::DuplicatedArticleId => StatusCode::CONFLICT,
                    CreateArticleError::Unauthorized => StatusCode::UNAUTHORIZED,
                    CreateArticleError::InvalidUtf8 => StatusCode::BAD_REQUEST,
                }
            }
        }
    }
}

impl ContainsHeaderMap for CreateArticleResult {
    type Iterator = Empty<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        empty()
    }
}

impl IntoPlainText for CreateArticleResult {
    fn into_plain_text(self) -> String {
        match self {
            Ok(s) => {
                let ArticleCreatedNotice { warnings, allocated_id } = s;
                let warnings = warnings
                    .into_iter()
                    .map(|a| a.to_string() + "\n")
                    .collect::<String>();

                format!("{warnings}OK, saved as {allocated_id}.")
            }
            Err(x) => {
                match x {
                    CreateArticleError::Unauthorized => {
                        "You must be authorized to perform this action.".to_string()
                    }
                    CreateArticleError::DuplicatedArticleId => {
                        "already exist. Please choose another one, or overwrite with PUT request.".to_string()
                    }
                    CreateArticleError::InvalidUtf8 => {
                        "text must be valid UTF-8".to_string()
                    }
                }
            }
        }
    }
}

impl HttpStatusCode for GetArticleResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(_) => StatusCode::OK,
            Err(y) => {
                match y {
                    GetArticleError::NoSuchArticleFoundById => StatusCode::NOT_FOUND,
                }
            }
        }
    }
}

impl ContainsHeaderMap for GetArticleResult {
    type Iterator = EitherIter<
        core::iter::Once<Pair>,
        Empty<Pair>,
        Pair,
    >;

    fn response_headers(&self) -> Self::Iterator {
        use std::iter::once as single_iter;

        self.as_ref().map_or(
            EitherIter::Right(empty()),
            |d| EitherIter::Left(
                single_iter(
                    // TODO: Having ETag is fun, right?
                    // compliant with RFC 7232 (HTTP/1.1 Conditional Requests) § 2.1.1
                    (
                        LAST_MODIFIED,
                        HeaderValueUpdateMethod::Overwrite(
                            HttpFormattedDate::new(d.metadata.updated_at).to_string().try_into().unwrap()
                        )
                    )
                )
            )
        )
    }
}

impl IntoPlainText for GetArticleResult {
    fn into_plain_text(self) -> String {
        match self {
            Ok(article) => {
                let OwnedMetadata { metadata: _, data } = article;
                data.content.into_inner()
            }
            Err(e) => {
                match e {
                    GetArticleError::NoSuchArticleFoundById => "Not found".to_string()
                }
            }
        }
    }
}

impl HttpStatusCode for UpdateArticleResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(_) => StatusCode::NO_CONTENT,
            Err(e) => {
                match e {
                    UpdateArticleError::InvalidBearerToken => StatusCode::UNAUTHORIZED,
                    UpdateArticleError::InvalidByteSequenceForUtf8(_) => StatusCode::BAD_REQUEST,
                    UpdateArticleError::ArticleNotFoundById => StatusCode::NOT_FOUND,
                }
            }
        }
    }
}

impl ContainsHeaderMap for UpdateArticleResult {
    type Iterator = Empty<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        core::iter::empty()
    }
}

impl IntoPlainText for UpdateArticleResult {
    fn into_plain_text(self) -> String {
        match self {
            Ok(_) => {
                "saved".to_string()
            }
            Err(e) => {
                match e {
                    UpdateArticleError::InvalidBearerToken => "You must be authorized to perform this action.".to_string(),
                    UpdateArticleError::ArticleNotFoundById => "Not found".to_string(),
                    UpdateArticleError::InvalidByteSequenceForUtf8(e) => format!("You must provide valid UTF-8 sequence: {e}")
                }
            }
        }
    }
}

impl HttpStatusCode for DeleteArticleResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(_) => StatusCode::NO_CONTENT,
            Err(e) => {
                match e {
                    DeleteArticleError::InvalidBearerToken => StatusCode::UNAUTHORIZED,
                    DeleteArticleError::NoSuchArticleFoundById => StatusCode::NOT_FOUND,
                }
            }
        }
    }
}

impl ContainsHeaderMap for DeleteArticleResult {
    type Iterator = Empty<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        core::iter::empty()
    }
}

impl IntoPlainText for DeleteArticleResult {
    fn into_plain_text(self) -> String {
        match self {
            Ok(_) => "deleted".to_string(),
            Err(e) => {
                match e {
                    DeleteArticleError::InvalidBearerToken => "You must be authorized to perform this action.".to_string(),
                    DeleteArticleError::NoSuchArticleFoundById => "Not found".to_string()
                }
            }
        }
    }
}

impl HttpStatusCode for ListArticleResponse {
    fn call_status_code(&self) -> StatusCode {
        StatusCode::OK
    }
}

impl ContainsHeaderMap for ListArticleResponse {
    type Iterator = VecIter<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        vec![
            (
                WARNING,
                HeaderValueUpdateMethod::Append(
                    "This endpoint will be removed in future major release".try_into().unwrap()
                )
            )
        ].into_iter()
    }
}

impl HttpStatusCode for ListArticleResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(e) => e.call_status_code(),
            Err(e) => {
                match *e {
                }
            }
        }
    }
}

impl ContainsHeaderMap for ListArticleResult {
    type Iterator = EitherIter<Empty<Pair>, VecIter<Pair>, Pair>;

    fn response_headers(&self) -> Self::Iterator {
        self.as_ref().map_or(
            EitherIter::Left(empty()),
            |x| EitherIter::Right(x.response_headers())
        )
    }
}

impl HttpStatusCode for ChangeArticleIdRequestResult {
    fn call_status_code(&self) -> StatusCode {
        match self {
            Ok(_) => StatusCode::NO_CONTENT,
            Err(e) => {
                match e {
                    ChangeArticleIdError::Unauthorized => StatusCode::UNAUTHORIZED,
                    ChangeArticleIdError::ArticleNotFoundById => StatusCode::NOT_FOUND,
                }
            }
        }
    }
}

impl ContainsHeaderMap for ChangeArticleIdRequestResult {
    type Iterator = Empty<Pair>;

    fn response_headers(&self) -> Self::Iterator {
        empty()
    }
}

impl IntoPlainText for ChangeArticleIdRequestResult {
    fn into_plain_text(self) -> String {
        match self {
            Ok(_) => {
                "The article was successfully renamed".to_string()
            }
            Err(e) => {
                match e {
                    ChangeArticleIdError::Unauthorized => "You must be authorized to perform this action.".to_string(),
                    ChangeArticleIdError::ArticleNotFoundById => "The article does not exist".to_string(),
                }
            }
        }
    }
}

pub(super) struct ArticleIdCollectionResponseRepr(pub(super) OwnedMetadata<ArticleIdSetMetadata, ArticleIdSet>);

impl HttpStatusCode for ArticleIdCollectionResponseRepr {
    fn call_status_code(&self) -> StatusCode {
        StatusCode::OK
    }
}

impl ContainsHeaderMap for ArticleIdCollectionResponseRepr {
    type Iterator = EitherIter<
        VecIter<(HeaderName, HeaderValueUpdateMethod)>,
        Empty<(HeaderName, HeaderValueUpdateMethod)>,
        (HeaderName, HeaderValueUpdateMethod)
    >;

    fn response_headers(&self) -> Self::Iterator {
        self.0.metadata.newest_updated_at.map_or(
            EitherIter::Right(empty()),
            |newest| {
                let date = HttpFormattedDate::new(newest.with_timezone(newest.offset()));
                let vec = vec![
                    (LAST_MODIFIED, HeaderValueUpdateMethod::Overwrite(date.to_string().try_into().unwrap()))
                ];
                EitherIter::Left(vec.into_iter())
            }
        )
    }
}

impl Serialize for ArticleIdCollectionResponseRepr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.data.serialize(serializer)
    }
}

pub(super) struct InternalErrorExposedRepr(pub(super) Box<dyn std::error::Error>);

impl HttpStatusCode for InternalErrorExposedRepr {
    fn call_status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl ContainsHeaderMap for InternalErrorExposedRepr {
    type Iterator = Empty<(HeaderName, HeaderValueUpdateMethod)>;

    fn response_headers(&self) -> Self::Iterator {
        empty()
    }
}

impl Serialize for InternalErrorExposedRepr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.to_string().serialize(serializer)
    }
}
