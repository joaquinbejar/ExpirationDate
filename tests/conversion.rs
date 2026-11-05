//! Integration tests covering conversion / formatting helpers on
//! [`ExpirationDate`] — `get_years`, `get_date`, `get_date_string`, the
//! `Default` impl, and related creation paths.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;
use positive::{Positive, pos_or_panic};
use std::cmp::Ordering;

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
    assert_eq!(tomorrow_date.cmp_at(&ten_days, now), Ordering::Less);
    // `Ord` needs no base: every `Days` comes before every `DateTime`.
    assert!(ten_days < tomorrow_date);
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

/// Asking for the days left no longer changes what a later `Days` resolves to.
#[test]
fn test_get_days_leaves_reference_datetime_unset() {
    ExpirationDate::set_reference_datetime(None);
    let date = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap());
    let _ = date.get_days().unwrap();
    let _ = date.get_years().unwrap();
    assert_eq!(ExpirationDate::get_reference_datetime(), None);
}

/// With a reference set, the clock forms resolve a `Days` as the pure form
/// does from that reference.
#[test]
fn test_get_date_with_reference_matches_get_date_at() {
    let reference = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    ExpirationDate::set_reference_datetime(Some(reference));
    let resolved = days.get_date().unwrap();
    ExpirationDate::set_reference_datetime(None);
    assert_eq!(resolved, days.get_date_at(reference).unwrap());
}

/// A date beyond what `DateTime` can hold is an error and not a panic, in the
/// clock forms as well.
#[test]
fn test_get_date_out_of_range_days_is_an_error() {
    for days in [1e9, 1e15, 1e20] {
        let expiration = ExpirationDate::Days(Positive::new(days).unwrap());
        assert!(expiration.get_date().is_err(), "{days} days");
        assert!(expiration.get_date_string().is_err(), "{days} days");
        assert_eq!(expiration.to_string(), "Invalid Expiration");
    }
}

/// A `Days` from `get_years_with_convention` keeps counting from the reference
/// for the target while the fraction starts at the clock, as it always has.
#[test]
fn test_get_years_with_convention_past_reference_gives_zero() {
    let reference = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    ExpirationDate::set_reference_datetime(Some(reference));
    let years = days.get_years_with_convention(expiration_date::conventions::Actual360);
    ExpirationDate::set_reference_datetime(None);
    assert_eq!(years.unwrap(), Positive::ZERO);
}
