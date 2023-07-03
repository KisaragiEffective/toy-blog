use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

#[test]
fn rfc7232_example() {
    let dt = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(1994, 11, 15).unwrap(),
        NaiveTime::from_hms_opt(12, 45, 26).unwrap()
    );

    assert_eq!(
        super::HttpFormattedDate::new(
            FixedOffset::east_opt(0).unwrap().from_utc_datetime(&dt)
        ).to_string(),
        "Tue, 15 Nov 1994 12:45:26 GMT"
    );
}

#[test]
fn rfc7232_example_jst() {
    assert_eq!(
        super::HttpFormattedDate::new(
            FixedOffset::east_opt(9 * 3600)
                .unwrap()
                .with_ymd_and_hms(1994, 11, 15, 21, 45, 26)
                .unwrap()
        ).to_string(),
        "Tue, 15 Nov 1994 12:45:26 GMT"
    );
}