//! Integration tests covering the `from_string` / `from_string_to_days`
//! parser family and multi-format acceptance for [`ExpirationDate`].

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use expiration_date::ExpirationDate;
use expiration_date::error::ExpirationDateError;
use std::error::Error;

#[test]
fn test_parsing_multi_formats() -> Result<(), Box<dyn Error>> {
    // Date-only formats that go through `NaiveDate::parse_from_str` (and so
    // produce DateTime variants). `"20251231"` is intentionally excluded:
    // v0.1.x semantics parse it as a `Positive` day count first, since
    // string-to-Positive parsing is attempted before any date format.
    let formats = [("2025-12-31", 2025, 12, 31), ("31-12-2025", 2025, 12, 31)];
    for (s, y, m, d) in formats {
        let exp = ExpirationDate::from_string(s)?;
        match exp {
            ExpirationDate::DateTime(dt) => {
                use chrono::Datelike;
                assert_eq!(dt.year(), y);
                assert_eq!(dt.month(), m);
                assert_eq!(dt.day(), d);
            }
            _ => return Err(format!("Failed format {}", s).into()),
        }
    }

    // The 8-digit-numeric branch matches when the input is treated as a
    // positive day count; verify that the helper accepts it as `Days`.
    let exp = ExpirationDate::from_string("20251231")?;
    assert!(matches!(exp, ExpirationDate::Days(_)));
    Ok(())
}

#[test]
fn test_from_string_valid_days() {
    let result = ExpirationDate::from_string("30.0");
    assert!(result.is_ok());
    match result.unwrap() {
        ExpirationDate::Days(days) => assert_eq!(days, 30.0),
        _ => panic!("Expected Days variant"),
    }
}

#[test]
fn test_from_string_rfc3339_z() {
    // RFC3339 with Z suffix should now parse successfully.
    let result = ExpirationDate::from_string("2024-12-31T00:00:00Z");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_full_month_space() {
    let result = ExpirationDate::from_string("30 january 2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_full_month_dash() {
    let result = ExpirationDate::from_string("30-january-2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_short_month_space() {
    let result = ExpirationDate::from_string("30 jan 2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_short_month_dash() {
    let result = ExpirationDate::from_string("30-jan-2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_uppercase_month() {
    let result = ExpirationDate::from_string("30 JAN 2025");
    assert!(result.is_ok());
    let result = ExpirationDate::from_string("30-Jan-2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_yyyymmdd() {
    let result = ExpirationDate::from_string("20250101");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_format_dd_mm_yyyy() {
    let result = ExpirationDate::from_string("30-01-2025");
    assert!(result.is_ok());
}

#[test]
fn test_from_string_invalid_format() {
    let result = ExpirationDate::from_string("invalid date");
    assert!(matches!(result, Err(ExpirationDateError::ParseError(_))));
}

#[test]
fn test_from_string_with_time_no_seconds() {
    let date_str = "2025-05-23T15:29";
    let result = ExpirationDate::from_string(date_str).unwrap();
    if let ExpirationDate::DateTime(dt) = result {
        assert_eq!(dt.format("%Y-%m-%dT%H:%M").to_string(), "2025-05-23T15:29");
    } else {
        panic!("Expected DateTime variant");
    }
}

#[test]
fn test_from_string_with_utc_suffix() {
    // Round-trip the `Display` output: "%Y-%m-%d %H:%M:%S UTC".
    let date_str = "2025-05-23 12:03:18 UTC";
    let result = ExpirationDate::from_string(date_str).unwrap();
    if let ExpirationDate::DateTime(dt) = result {
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2025-05-23 12:03:18"
        );
    } else {
        panic!("Expected DateTime variant");
    }
}

#[test]
fn test_from_string_to_days_for_date() {
    let result = ExpirationDate::from_string_to_days("2099-12-31").unwrap();
    match result {
        ExpirationDate::Days(_) => {}
        _ => panic!("Expected Days variant"),
    }
}

#[test]
fn test_from_string_to_days_for_number() {
    let result = ExpirationDate::from_string_to_days("42.5").unwrap();
    match result {
        ExpirationDate::Days(d) => assert!((d.to_f64() - 42.5).abs() < f64::EPSILON),
        _ => panic!("Expected Days variant"),
    }
}
