//! Property-based tests for the [`ExpirationDate::from_string`] parser family.
//!
//! Each property is configured for 256 cases by default; set the
//! `PROPTEST_CASES` environment variable to override.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use expiration_date::EPSILON;
use expiration_date::ExpirationDate;
use expiration_date::error::ExpirationDateError;
use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Strategy producing a valid `(year, month, day, hour, min, sec)` tuple.
///
/// Uses `NaiveDate::from_ymd_opt` as a filter so only real calendar dates
/// survive (leap years, 30/31-day months, etc.).
fn valid_datetime_components() -> impl Strategy<Value = (i32, u32, u32, u32, u32, u32)> {
    (
        1970i32..2200,
        1u32..=12,
        1u32..=31,
        0u32..24,
        0u32..60,
        0u32..60,
    )
        .prop_filter("valid calendar date", |(y, m, d, _, _, _)| {
            NaiveDate::from_ymd_opt(*y, *m, *d).is_some()
        })
}

/// Strategy producing a valid `(year, month, day)` tuple.
fn valid_date_components() -> impl Strategy<Value = (i32, u32, u32)> {
    (1970i32..2200, 1u32..=12, 1u32..=31).prop_filter("valid calendar date", |(y, m, d)| {
        NaiveDate::from_ymd_opt(*y, *m, *d).is_some()
    })
}

/// Strategy producing strings that should never match any known format.
///
/// Builds lowercase ASCII alphabetic strings of length 5..20. These contain
/// no digits, no `-`, no `T`, and no month abbreviations of the right shape,
/// so they should not parse as `Positive`, RFC3339, `YYYYMMDD`, `DD-MM-YYYY`,
/// or any of the named-month formats.
fn malformed_string_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::char::range('a', 'z'), 5..20)
        .prop_map(|chars| chars.into_iter().collect::<String>())
        .prop_filter("not a coincidental month-name format", |s| {
            // Guard against the unlikely accident of producing a string that
            // matches a `%d %B %Y` style format (which we already exclude by
            // not containing digits or spaces). Alphabetic-only strings with
            // no separators cannot match any format, so this is belt-and-
            // braces; keep it cheap.
            !s.is_empty()
        })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    /// RFC3339 round-trip: building a `DateTime<Utc>` from proptest-generated
    /// components, formatting with `%Y-%m-%dT%H:%M:%SZ`, and feeding it to
    /// `from_string` must yield the same absolute instant.
    #[test]
    fn prop_rfc3339_round_trip(
        (year, month, day, hour, min, sec) in valid_datetime_components()
    ) {
        let dt = Utc
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .single();
        prop_assume!(dt.is_some());
        let dt = dt.unwrap();
        let s = dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let parsed = ExpirationDate::from_string(&s);
        prop_assert!(parsed.is_ok(), "RFC3339 parse failed: {s:?} -> {parsed:?}");

        match parsed.unwrap() {
            ExpirationDate::DateTime(parsed_dt) => {
                prop_assert_eq!(parsed_dt, dt);
            }
            ExpirationDate::Days(_) => {
                prop_assert!(false, "expected DateTime variant for RFC3339 input {s:?}");
            }
        }
    }

    /// `YYYYMMDD`: well-formed 8-digit calendar strings parse successfully.
    ///
    /// NOTE (v0.1.x semantics): `from_string` first attempts
    /// `s.parse::<Positive>()`, and an all-digit string is a valid positive
    /// number. Therefore the parser returns the `Days` variant, not the
    /// `DateTime` variant, for 8-digit inputs. This property asserts the
    /// observed behaviour so any future semver-gated change is flagged.
    #[test]
    fn prop_yyyymmdd_parses_as_days_variant(
        (year, month, day) in valid_date_components()
    ) {
        let s = format!("{year:04}{month:02}{day:02}");
        let parsed = ExpirationDate::from_string(&s);
        prop_assert!(parsed.is_ok(), "YYYYMMDD parse failed: {s:?} -> {parsed:?}");

        match parsed.unwrap() {
            ExpirationDate::Days(days) => {
                // The string parsed as a positive day count.
                let expected = Decimal::from_str(&s).unwrap();
                prop_assert!(
                    (days.to_dec() - expected).abs() < EPSILON,
                    "Days({days}) != {s}"
                );
            }
            ExpirationDate::DateTime(_) => {
                prop_assert!(
                    false,
                    "expected Days variant for 8-digit numeric input {s:?} (v0.1.x semantics)"
                );
            }
        }
    }

    /// `DD-MM-YYYY`: well-formed date strings parse to the `DateTime` variant.
    #[test]
    fn prop_dd_mm_yyyy_parses_as_datetime(
        (year, month, day) in valid_date_components()
    ) {
        let s = format!("{day:02}-{month:02}-{year:04}");
        let parsed = ExpirationDate::from_string(&s);
        prop_assert!(parsed.is_ok(), "DD-MM-YYYY parse failed: {s:?} -> {parsed:?}");

        match parsed.unwrap() {
            ExpirationDate::DateTime(dt) => {
                prop_assert_eq!(dt.year(), year);
                prop_assert_eq!(dt.month(), month);
                prop_assert_eq!(dt.day(), day);
                // Parser normalises date-only inputs to 18:30:00 UTC.
                prop_assert_eq!(dt.hour(), 18);
                prop_assert_eq!(dt.minute(), 30);
                prop_assert_eq!(dt.second(), 0);
            }
            ExpirationDate::Days(_) => {
                prop_assert!(
                    false,
                    "expected DateTime variant for DD-MM-YYYY input {s:?}"
                );
            }
        }
    }

    /// Positive numeric days: finite positive `f64` values in `(0.0, 1e9]`
    /// formatted as decimal strings parse to the `Days` variant and round-
    /// trip within `EPSILON`.
    #[test]
    fn prop_positive_days_round_trip(value in 1e-9f64..1e9f64) {
        // Use a fixed-precision decimal representation so the intermediate
        // string is a clean decimal (no scientific notation).
        let s = format!("{value:.9}");

        let parsed = ExpirationDate::from_string(&s);
        prop_assert!(parsed.is_ok(), "positive-days parse failed: {s:?} -> {parsed:?}");

        match parsed.unwrap() {
            ExpirationDate::Days(days) => {
                let expected = Decimal::from_str(&s).unwrap();
                let diff = (days.to_dec() - expected).abs();
                prop_assert!(
                    diff < EPSILON,
                    "Days({days}) not within EPSILON of {s} (diff={diff})"
                );
            }
            ExpirationDate::DateTime(_) => {
                prop_assert!(false, "expected Days variant for numeric input {s:?}");
            }
        }
    }

    /// Malformed inputs must return a `ParseError` (or a `ChronoParseError`
    /// if an internal chrono try-parse leaked through). Random lowercase
    /// alphabetic strings cannot match any known format.
    #[test]
    fn prop_malformed_inputs_reject(s in malformed_string_strategy()) {
        let parsed = ExpirationDate::from_string(&s);
        prop_assert!(
            parsed.is_err(),
            "expected error for malformed input {s:?}, got {parsed:?}"
        );
        match parsed.unwrap_err() {
            ExpirationDateError::ParseError(_)
            | ExpirationDateError::ChronoParseError(_)
            | ExpirationDateError::PositiveError(_)
            | ExpirationDateError::ParseIntError(_) => {}
            other => {
                prop_assert!(
                    false,
                    "unexpected error variant for malformed input {s:?}: {other:?}"
                );
            }
        }
    }
}
