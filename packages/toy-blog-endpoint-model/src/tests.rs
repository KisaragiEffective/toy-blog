use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use super::fmt_http_date;

#[test]
fn rfc7232_example() {
    let dt = NaiveDateTime::new(
        NaiveDate::from_ymd(1994, 11, 15),
        NaiveTime::from_hms_opt(12, 45, 26).unwrap()
    );

    assert_eq!(
        super::HttpFormattedDate::new(
            FixedOffset::east(0).from_utc_datetime(&dt)
        ).to_string(),
        "Tue, 15 Nov 1994 12:45:26 GMT"
    )
}

#[test]
fn rfc7232_example_jst() {
    assert_eq!(
        super::HttpFormattedDate::new(
            FixedOffset::east(9 * 3600).ymd(1994, 11, 15).and_hms(21, 45, 26)
        ).to_string(),
        "Tue, 15 Nov 1994 12:45:26 GMT"
    )
}