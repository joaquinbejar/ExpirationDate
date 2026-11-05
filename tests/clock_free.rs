//! Integration tests for what a build without the `clock` feature keeps: the
//! conversions that take the time as an argument, parsing and `Display`.
//!
//! This file runs in both builds, so it holds whether or not `clock` is on.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{DateTime, Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;
use expiration_date::conventions::{Actual360, Actual365Fixed};
use expiration_date::error::ExpirationDateError;
use positive::{Positive, pos_or_panic};

fn base() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap()
}

/// A reference far from `base`, so a result that used it would be visible.
fn distant_reference() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(1990, 6, 15, 12, 0, 0).unwrap()
}

/// Runs `check` with a reference set and clears it again.
fn with_reference<T>(reference: DateTime<Utc>, check: impl FnOnce() -> T) -> T {
    ExpirationDate::set_reference_datetime(Some(reference));
    let result = check();
    ExpirationDate::set_reference_datetime(None);
    result
}

#[test]
fn test_get_date_at_days_counts_from_base() {
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    assert_eq!(
        days.get_date_at(base()).unwrap(),
        base() + Duration::days(30)
    );
}

#[test]
fn test_get_date_at_days_drops_fraction_of_a_day() {
    let days = ExpirationDate::Days(pos_or_panic!(1.75));
    assert_eq!(
        days.get_date_at(base()).unwrap(),
        base() + Duration::days(1)
    );
}

#[test]
fn test_get_date_at_datetime_ignores_base() {
    let instant = Utc.with_ymd_and_hms(2024, 3, 20, 16, 0, 0).unwrap();
    let date = ExpirationDate::DateTime(instant);
    assert_eq!(date.get_date_at(base()).unwrap(), instant);
}

#[test]
fn test_get_date_at_ignores_reference_datetime() {
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    let resolved = with_reference(distant_reference(), || days.get_date_at(base()).unwrap());
    assert_eq!(resolved, base() + Duration::days(30));
}

/// `Days(1e9)` overflows the addition to `DateTime` and `Days(1e15)` overflows
/// the span itself; neither may panic.
#[test]
fn test_get_date_at_unrepresentable_instant_is_overflow_error() {
    for days in [1e9, 1e15] {
        let result = ExpirationDate::Days(Positive::new(days).unwrap()).get_date_at(base());
        assert!(
            matches!(result, Err(ExpirationDateError::ArithmeticOverflow(_))),
            "{days} days gave {result:?}"
        );
    }
}

#[test]
fn test_get_date_at_day_count_beyond_i64_is_error() {
    let result = ExpirationDate::Days(Positive::MAX).get_date_at(base());
    assert!(matches!(result, Err(ExpirationDateError::PositiveError(_))));
}

#[test]
fn test_get_days_at_days_returns_own_count() {
    let days = ExpirationDate::Days(pos_or_panic!(42.5));
    assert_eq!(days.get_days_at(base()).unwrap(), pos_or_panic!(42.5));
}

#[test]
fn test_get_days_at_datetime_counts_from_now() {
    let date = ExpirationDate::DateTime(base() + Duration::hours(36));
    assert_eq!(date.get_days_at(base()).unwrap(), pos_or_panic!(1.5));
}

#[test]
fn test_get_days_at_past_datetime_is_zero() {
    let date = ExpirationDate::DateTime(base() - Duration::days(1));
    assert_eq!(date.get_days_at(base()).unwrap(), Positive::ZERO);
    let exactly_now = ExpirationDate::DateTime(base());
    assert_eq!(exactly_now.get_days_at(base()).unwrap(), Positive::ZERO);
}

/// The answer depends on the argument and on nothing else: the same call gives
/// the same value, and a later `now` gives fewer days.
#[test]
fn test_get_days_at_is_a_function_of_its_argument() {
    let date = ExpirationDate::DateTime(base() + Duration::days(10));
    let first = date.get_days_at(base()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(date.get_days_at(base()).unwrap(), first);
    let later = date.get_days_at(base() + Duration::days(4)).unwrap();
    assert_eq!(later, pos_or_panic!(6.0));
}

#[test]
fn test_get_years_at_days_divides_by_365() {
    let days = ExpirationDate::Days(pos_or_panic!(182.5));
    assert_eq!(days.get_years_at(base()).unwrap(), pos_or_panic!(0.5));
}

#[test]
fn test_get_years_at_datetime_uses_actual_365_fixed() {
    let date = ExpirationDate::DateTime(base() + Duration::days(365));
    assert_eq!(date.get_years_at(base()).unwrap(), Positive::ONE);
}

#[test]
fn test_get_years_at_past_datetime_is_zero() {
    let date = ExpirationDate::DateTime(base() - Duration::days(30));
    assert_eq!(date.get_years_at(base()).unwrap(), Positive::ZERO);
}

#[test]
fn test_get_years_with_convention_at_applies_the_convention() {
    let date = ExpirationDate::DateTime(base() + Duration::days(360));
    let on_360 = date.get_years_with_convention_at(Actual360, base());
    let on_365 = date.get_years_with_convention_at(Actual365Fixed, base());
    assert_eq!(on_360.unwrap(), Positive::ONE);
    assert!(on_365.unwrap() < Positive::ONE);
}

/// A `Days` counts from `now`, not from a reference that lies elsewhere.
#[test]
fn test_get_years_with_convention_at_days_ignores_reference_datetime() {
    let days = ExpirationDate::Days(pos_or_panic!(360.0));
    let years = with_reference(distant_reference(), || {
        days.get_years_with_convention_at(Actual360, base())
            .unwrap()
    });
    assert_eq!(years, Positive::ONE);
}

/// The date of a `Days` counts from 18:30 on that day, so the hour of `today`
/// does not move it.
#[test]
fn test_get_date_string_at_days_counts_from_the_day_of_today() {
    let days = ExpirationDate::Days(pos_or_panic!(1.0));
    let early = Utc.with_ymd_and_hms(2030, 1, 31, 0, 0, 1).unwrap();
    let late = Utc.with_ymd_and_hms(2030, 1, 31, 23, 59, 59).unwrap();
    assert_eq!(days.get_date_string_at(early).unwrap(), "2030-02-01");
    assert_eq!(days.get_date_string_at(late).unwrap(), "2030-02-01");
}

#[test]
fn test_get_date_string_at_datetime_ignores_today() {
    let date = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap());
    assert_eq!(date.get_date_string_at(base()).unwrap(), "2024-12-31");
}

/// A conversion leaves no trace: nothing in it writes the reference datetime.
#[test]
fn test_conversions_at_do_not_write_reference_datetime() {
    ExpirationDate::set_reference_datetime(None);
    let date = ExpirationDate::DateTime(base() + Duration::days(10));
    let days = ExpirationDate::Days(pos_or_panic!(10.0));
    for expiration in [date, days] {
        let _ = expiration.get_date_at(base()).unwrap();
        let _ = expiration.get_days_at(base()).unwrap();
        let _ = expiration.get_years_at(base()).unwrap();
        let _ = expiration.get_date_string_at(base()).unwrap();
        let _ = expiration.to_string();
        assert_eq!(ExpirationDate::get_reference_datetime(), None);
    }
}

/// Every date shape of the grammar parses without setting the reference, and
/// one set by the caller survives.
#[test]
fn test_from_string_does_not_write_reference_datetime() {
    let inputs = [
        "2025-01-01T00:00:00Z",
        "2025-01-01 12:00:00 UTC",
        "2025-05-23T15:29",
        "2025-01-01",
        "01-01-2025",
        "30 jan 2025",
        "30-jan-2025",
        "30 january 2025",
        "30-january-2025",
    ];

    ExpirationDate::set_reference_datetime(None);
    for input in inputs {
        let parsed = ExpirationDate::from_string(input).unwrap();
        assert!(matches!(parsed, ExpirationDate::DateTime(_)), "{input}");
        assert_eq!(ExpirationDate::get_reference_datetime(), None, "{input}");
    }

    let reference = distant_reference();
    with_reference(reference, || {
        for input in inputs {
            let _ = ExpirationDate::from_string(input).unwrap();
            assert_eq!(
                ExpirationDate::get_reference_datetime(),
                Some(reference),
                "{input}"
            );
        }
    });
}

#[test]
fn test_from_string_to_days_at_date_counts_from_now() {
    let now = Utc.with_ymd_and_hms(2025, 1, 1, 18, 30, 0).unwrap();
    let parsed = ExpirationDate::from_string_to_days_at("2025-01-11", now).unwrap();
    assert_eq!(parsed, ExpirationDate::Days(pos_or_panic!(10.0)));
}

#[test]
fn test_from_string_to_days_at_past_date_is_zero_days() {
    let parsed = ExpirationDate::from_string_to_days_at("2020-01-01", base()).unwrap();
    assert_eq!(parsed, ExpirationDate::Days(Positive::ZERO));
}

#[test]
fn test_from_string_to_days_at_number_keeps_the_count() {
    let parsed = ExpirationDate::from_string_to_days_at("42.5", base()).unwrap();
    assert_eq!(parsed, ExpirationDate::Days(pos_or_panic!(42.5)));
}

#[test]
fn test_from_string_to_days_at_invalid_input_is_error() {
    assert!(ExpirationDate::from_string_to_days_at("not a date", base()).is_err());
}

#[test]
fn test_display_datetime_prints_its_instant() {
    let date = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2024, 3, 20, 16, 0, 0).unwrap());
    assert_eq!(date.to_string(), "2024-03-20 16:00:00 UTC");
}

#[test]
fn test_display_days_counts_from_reference_datetime() {
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    let text = with_reference(base(), || days.to_string());
    assert_eq!(text, "2030-01-31 00:00:00 UTC");
}

#[test]
fn test_display_days_past_datetime_range_is_invalid() {
    let days = ExpirationDate::Days(pos_or_panic!(1e9));
    let text = with_reference(base(), || days.to_string());
    assert_eq!(text, "Invalid Expiration");
}

/// With no clock and no reference there is no base, so the count is printed.
#[cfg(not(feature = "clock"))]
#[test]
fn test_display_days_without_clock_or_reference_prints_the_count() {
    ExpirationDate::set_reference_datetime(None);
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    assert_eq!(days.to_string(), "30 days");
}
