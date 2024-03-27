use std::error::Error;
use std::future::Ready;
use std::str::FromStr;
use actix_web::{FromRequest, HttpRequest};
use actix_web::dev::Payload;
use actix_web::http::header::{HeaderValue, ToStrError};
use chrono::{DateTime, FixedOffset, ParseError, TimeZone};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LastModifiedParseError {
    #[error("given value does not fit in ASCII: {0}")]
    NotInAscii(#[from] ToStrError),
    #[error("chrono reports parse failure: {0}")]
    Chrono(#[from] ParseError)
}

#[derive(Eq, PartialEq, Debug)]
pub struct LastModified(pub DateTime<FixedOffset>);

impl FromStr for LastModified {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = chrono::NaiveDateTime::parse_from_str(s, "%a, %d %b %Y %H:%M:%S GMT")?;
        Ok(Self(FixedOffset::east_opt(0).unwrap().from_utc_datetime(&x)))
    }
}

impl TryFrom<&HeaderValue> for LastModified {
    type Error = LastModifiedParseError;

    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        Ok(value.to_str()?.parse()?)
    }
}

impl FromRequest for LastModified {
    type Error = LastModifiedExtractionError;
    type Future = Ready<Result<LastModified, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let w = req.headers().get("Last-Modified")
            .ok_or(LastModifiedExtractionError::NotFound);
        let w = match w {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(e)),
        };
        
        let r = Self::try_from(w);
        let r = match r {
            Ok(t) => t,
            Err(e) => return std::future::ready(Err(LastModifiedExtractionError::from(e))),
        };
        
        std::future::ready(Ok(r))
    }
}

#[derive(Error, Debug)]
pub enum LastModifiedExtractionError {
    #[error("request does not have Last-Modified header")]
    NotFound,
    #[error("header value is malformed: {0}")]
    ParseFailure(#[from] LastModifiedParseError),
}

impl actix_web::ResponseError for LastModifiedExtractionError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use crate::service::rest::header::LastModified;

    #[test]
    fn rfc_7232_example() {
        let parsed = LastModified::from_str("Tue, 15 Nov 1994 12:45:26 GMT").expect("failed to parse");
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
    fn xy() {
        let _ = LastModified::from_str("Tue, 15 Nov 1994 12:45:26 GMT1").expect("failed to parse");
    }
    #[test]
    #[should_panic]
    fn xyz() {
        let _ = LastModified::from_str("Tue").expect("failed to parse");
    }
}