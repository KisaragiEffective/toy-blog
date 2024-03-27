use std::fmt::Display;
use std::future::Ready;
use std::str::FromStr;
use actix_web::{FromRequest, HttpRequest};
use actix_web::dev::Payload;
use actix_web::http::header::{HeaderValue, ToStrError};
use chrono::{DateTime, FixedOffset, ParseError, TimeZone};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpDateParseError {
    #[error("given value does not fit in ASCII: {0}")]
    NotInAscii(#[from] ToStrError),
    #[error("chrono reports parse failure: {0}")]
    Chrono(#[from] ParseError)
}

const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

#[derive(Eq, PartialEq, Debug)]
pub struct HttpDate(pub DateTime<FixedOffset>);

impl FromStr for HttpDate {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = chrono::NaiveDateTime::parse_from_str(s, HTTP_DATE_FORMAT)?;
        Ok(Self(FixedOffset::east_opt(0).unwrap().from_utc_datetime(&x)))
    }
}

impl TryFrom<&str> for HttpDate {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for HttpDate {
    fn from(value: DateTime<Tz>) -> Self {
        Self(value.with_timezone(&FixedOffset::east_opt(0).unwrap()))
    }
}

impl Display for HttpDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format(HTTP_DATE_FORMAT))
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct LastModified(pub HttpDate);

impl TryFrom<&HeaderValue> for LastModified {
    type Error = HttpDateParseError;

    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        Ok(Self(value.to_str()?.parse()?))
    }
}

impl FromRequest for LastModified {
    type Error = HttpDateExtractionError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let w = req.headers().get("Last-Modified")
            .ok_or(HttpDateExtractionError::NotFound);
        let w = match w {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(e)),
        };

        let r = Self::try_from(w);
        let r = match r {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(HttpDateExtractionError::from(e))),
        };

        std::future::ready(Ok(r))
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct IfModifiedSince(pub HttpDate);

impl TryFrom<&HeaderValue> for IfModifiedSince {
    type Error = HttpDateParseError;

    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        Ok(Self(value.to_str()?.parse()?))
    }
}

impl FromRequest for IfModifiedSince {
    type Error = HttpDateExtractionError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let w = req.headers().get("If-Modified-Since")
            .ok_or(HttpDateExtractionError::NotFound);
        let w = match w {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(e)),
        };

        let r = Self::try_from(w);
        let r = match r {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(HttpDateExtractionError::from(e))),
        };

        std::future::ready(Ok(r))
    }
}

#[derive(Error, Debug)]
pub enum HttpDateExtractionError {
    #[error("request does not have Last-Modified header")]
    NotFound,
    #[error("header value is malformed: {0}")]
    ParseFailure(#[from] HttpDateParseError),
}

impl actix_web::ResponseError for HttpDateExtractionError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use crate::service::rest::header::{HttpDate};

    #[test]
    fn rfc_7232_example_should_able_to_be_parsed() {
        let parsed = HttpDate::from_str("Tue, 15 Nov 1994 12:45:26 GMT").expect("failed to parse");
        let expected = FixedOffset::east_opt(0).expect("timezone").from_utc_datetime(
            &NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1994, 11, 15).expect("naive date"),
                NaiveTime::from_hms_opt(12, 45, 26).expect("naive time")
            )
        );

        assert_eq!(parsed.0, expected);
    }
    #[test]
    #[should_panic]
    fn too_long_should_fail() {
        let _ = HttpDate::from_str("Tue, 15 Nov 1994 12:45:26 GMT1").expect("failed to parse");
    }
    #[test]
    #[should_panic]
    fn too_short_should_fail() {
        let _ = HttpDate::from_str("Tue").expect("failed to parse");
    }
}