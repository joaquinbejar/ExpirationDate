//! Integration tests covering conversion / formatting helpers on
//! [`ExpirationDate`] — `get_years`, `get_date`, `get_date_string`, the
//! `Default` impl, and related creation paths.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;
use positive::{Positive, pos_or_panic};

#[test]
fn test_expiration_date_days() {
    let days_in_year = pos_or_panic!(365.0);
    let expiration = ExpirationDate::Days(days_in_year);
    assert_eq!(expiration.get_years().unwrap(), 1.0);

    let expiration = ExpirationDate::Days(pos_or_panic!(182.5));
    assert_eq!(expiration.get_years().unwrap(), 0.5);

    let expiration = ExpirationDate::Days(Positive::ZERO);
    assert_eq!(expiration.get_years().unwrap(), 0.0);
}

#[test]
fn test_expiration_date_datetime() {
    // Test for a date exactly one year in the future
    let one_year_future = Utc::now() + Duration::days(365);
    let expiration = ExpirationDate::DateTime(one_year_future);
    assert!((expiration.get_years().unwrap().to_f64() - 1.0).abs() < 0.01);

    // Test for a date 6 months in the future
    let six_months_future = Utc::now() + Duration::days(182);
    let expiration = ExpirationDate::DateTime(six_months_future);
    assert!((expiration.get_years().unwrap().to_f64() - 0.5).abs() < 0.01);
}

#[test]
fn test_expiration_date_datetime_specific() {
    // Use a multi-day offset so the integer day count used by
    // Actual/365 Fixed yields a non-zero year fraction even after the
    // small delay between constructing `specific_date` and the internal
    // `Utc::now()` call inside `get_years()`.
    let specific_date = Utc::now() + Duration::days(7);
    let expiration = ExpirationDate::DateTime(specific_date);
    assert!(expiration.get_years().unwrap() > Positive::ZERO);
}

#[test]
fn test_get_date_from_datetime() {
    let future_date = Utc::now() + Duration::days(60);
    let expiration = ExpirationDate::DateTime(future_date);
    let result = expiration.get_date().unwrap();
    assert_eq!(result, future_date);
}

#[test]
fn test_get_date_from_past_datetime() {
    let past_date = Utc::now() - Duration::days(30);
    let expiration = ExpirationDate::DateTime(past_date);
    let result = expiration.get_date().unwrap();
    assert_eq!(result, past_date);
}

#[test]
fn test_positive_days() {
    let days_in_year = pos_or_panic!(365.0);
    let expiration = ExpirationDate::Days(days_in_year);
    let years = expiration.get_years().unwrap();
    assert_eq!(years, 1.0);
}

#[test]
fn test_comparisons() {
    // Make sure no leftover reference datetime affects comparisons.
    ExpirationDate::set_reference_datetime(None);

    let one_day = ExpirationDate::Days(Positive::ONE);
    let less_than_one_day = ExpirationDate::Days(pos_or_panic!(0.99));
    assert!(less_than_one_day < one_day);

    let now = Utc::now();
    let future = now + Duration::days(1);
    let past = now - Duration::days(1);
    let future_date = ExpirationDate::DateTime(future);
    let past_date = ExpirationDate::DateTime(past);
    assert!(future_date > past_date);

    let ten_days = ExpirationDate::Days(Positive::TEN);
    let tomorrow = Utc::now() + Duration::days(1);
    let tomorrow_date = ExpirationDate::DateTime(tomorrow);
    assert!(tomorrow_date < ten_days);
}

#[test]
fn test_default() {
    let default = ExpirationDate::default();
    match default {
        ExpirationDate::Days(days) => assert_eq!(days, pos_or_panic!(365.0)),
        _ => panic!("Expected Days variant"),
    }
}

#[test]
fn test_get_date_string_days() {
    // Clear any leftover reference datetime so the relative computation
    // uses today's date.
    ExpirationDate::set_reference_datetime(None);
    let today = Utc::now();
    let expiration = ExpirationDate::Days(pos_or_panic!(30.0));
    let date_str = expiration.get_date_string().unwrap();
    let expected_date = (today + Duration::days(30)).format("%Y-%m-%d").to_string();
    assert_eq!(date_str, expected_date);
}

#[test]
fn test_get_date_string_datetime() {
    let specific_date = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
    let expiration = ExpirationDate::DateTime(specific_date);
    assert_eq!(expiration.get_date_string().unwrap(), "2024-12-31");
}
