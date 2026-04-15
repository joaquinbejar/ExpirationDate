//! Cross-cutting tests for the `expiration_date` crate.
//!
//! Restored from v0.1.x to back-fill the coverage that was dropped during the
//! v0.2.0 conventions refactor (issue #3).

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use crate::ExpirationDate;
use crate::conventions::{Actual360, DayCount, Thirty360US};
use chrono::{Duration, TimeZone, Utc};
use positive::Positive;
use positive::constants::DAYS_IN_A_YEAR;
use std::error::Error;

#[test]
fn test_actual_365_fixed() -> Result<(), Box<dyn Error>> {
    let exp = ExpirationDate::Days(DAYS_IN_A_YEAR);
    let years = exp.get_years()?;
    assert!((years.to_f64() - 1.0).abs() < 1e-10);
    Ok(())
}

#[test]
fn test_actual_360() -> Result<(), Box<dyn Error>> {
    let exp = ExpirationDate::Days(Positive::new(360.0)?);
    let years = exp.get_years_with_convention(Actual360)?;
    assert!((years.to_f64() - 1.0).abs() < 1e-10);
    Ok(())
}

#[test]
fn test_thirty_360_us() -> Result<(), Box<dyn Error>> {
    let start = Utc
        .with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
        .single()
        .ok_or("date error")?;
    let end = Utc
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .single()
        .ok_or("date error")?;
    let conv = Thirty360US;
    let days = conv.day_count(&start, &end)?;
    assert!((days - 360.0).abs() < f64::EPSILON);
    Ok(())
}

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
fn test_comparisons_and_sorting() {
    let d1 = ExpirationDate::Days(Positive::ONE);
    let d2 = ExpirationDate::Days(Positive::TEN);
    let mut list = [d2, d1];
    list.sort();
    assert_eq!(list.first(), Some(&d1));
}

#[test]
fn test_reference_datetime() -> Result<(), Box<dyn Error>> {
    let ref_dt = Utc
        .with_ymd_and_hms(2020, 1, 1, 0, 0, 0)
        .single()
        .ok_or("date error")?;
    ExpirationDate::set_reference_datetime(Some(ref_dt));
    let exp = ExpirationDate::Days(Positive::ONE);
    let resolved = exp.get_date()?;
    assert_eq!(resolved, ref_dt + Duration::days(1));
    ExpirationDate::set_reference_datetime(None);
    Ok(())
}

mod tests_expiration_date {
    use super::*;
    use positive::pos_or_panic;

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
}

mod tests_formatting {
    use super::*;
    use positive::pos_or_panic;

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
}

mod tests_from_string {
    use super::*;
    use crate::error::ExpirationDateError;

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
}

mod tests_serialization {
    use super::*;
    use positive::pos_or_panic;

    #[test]
    fn test_expiration_date_days_serialization() {
        let days = pos_or_panic!(30.0);
        let expiration = ExpirationDate::Days(days);
        let serialized = serde_json::to_string(&expiration).unwrap();
        assert_eq!(serialized, r#"{"days":30.0}"#);
    }

    #[test]
    fn test_expiration_date_days_deserialization() {
        let json = r#"{"days": 30.0}"#;
        let deserialized: ExpirationDate = serde_json::from_str(json).unwrap();
        match deserialized {
            ExpirationDate::Days(days) => assert_eq!(days, pos_or_panic!(30.0)),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_expiration_date_datetime_serialization() {
        let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let expiration = ExpirationDate::DateTime(dt);
        let serialized = serde_json::to_string(&expiration).unwrap();
        assert_eq!(serialized, r#"{"datetime":"2025-01-01T00:00:00Z"}"#);
    }

    #[test]
    fn test_expiration_date_datetime_deserialization() {
        let json = r#"{"datetime": "2025-01-01T00:00:00Z"}"#;
        let deserialized: ExpirationDate = serde_json::from_str(json).unwrap();
        match deserialized {
            ExpirationDate::DateTime(dt) => {
                assert_eq!(dt, Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
            }
            _ => panic!("Expected DateTime variant"),
        }
    }

    #[test]
    fn test_expiration_date_roundtrip_days() {
        let original = ExpirationDate::Days(pos_or_panic!(365.0));
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ExpirationDate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_expiration_date_roundtrip_datetime() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        let original = ExpirationDate::DateTime(dt);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ExpirationDate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_invalid_datetime_deserialization() {
        let json = r#"{"datetime":{"0":"invalid-date"}}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_days_deserialization() {
        let json = r#"{"days":{"0":-30.0}}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_variant_deserialization() {
        let json = r#"{"invalid":{"0":30}}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_days_deserialization_rejected() {
        let json = r#"{"days":-1.0}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(
            result.is_err(),
            "negative days should be rejected by Positive"
        );
    }

    #[test]
    fn test_duplicate_days_field_deserialization() {
        let json = r#"{"days":1.0,"days":2.0}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(result.is_err(), "duplicate `days` field must error");
    }

    #[test]
    fn test_duplicate_datetime_field_deserialization() {
        let json = r#"{"datetime":"2025-01-01T00:00:00Z","datetime":"2025-01-02T00:00:00Z"}"#;
        let result = serde_json::from_str::<ExpirationDate>(json);
        assert!(result.is_err(), "duplicate `datetime` field must error");
    }
}

mod tests_hash {
    use super::*;

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_same_days_expiration_same_hash() {
        let exp1 = ExpirationDate::Days(Positive::new(30.0).unwrap());
        let exp2 = ExpirationDate::Days(Positive::new(30.0).unwrap());
        assert_eq!(calculate_hash(&exp1), calculate_hash(&exp2));
    }

    #[test]
    fn test_different_days_expiration_different_hash() {
        let exp1 = ExpirationDate::Days(Positive::new(30.0).unwrap());
        let exp2 = ExpirationDate::Days(Positive::new(45.0).unwrap());
        assert_ne!(calculate_hash(&exp1), calculate_hash(&exp2));
    }

    #[test]
    fn test_same_datetime_expiration_same_hash() {
        let date1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let exp1 = ExpirationDate::DateTime(date1);
        let exp2 = ExpirationDate::DateTime(date2);
        assert_eq!(calculate_hash(&exp1), calculate_hash(&exp2));
    }

    #[test]
    fn test_different_datetime_expiration_different_hash() {
        let date1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        let exp1 = ExpirationDate::DateTime(date1);
        let exp2 = ExpirationDate::DateTime(date2);
        assert_ne!(calculate_hash(&exp1), calculate_hash(&exp2));
    }

    #[test]
    fn test_different_variants_different_hash() {
        let date = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let exp1 = ExpirationDate::Days(Positive::new(30.0).unwrap());
        let exp2 = ExpirationDate::DateTime(date);
        assert_ne!(calculate_hash(&exp1), calculate_hash(&exp2));
    }

    #[test]
    fn test_hash_consistency_over_time() {
        let date = Utc::now();
        let exp = ExpirationDate::DateTime(date);
        let hash1 = calculate_hash(&exp);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let hash2 = calculate_hash(&exp);
        assert_eq!(hash1, hash2, "Hash should be consistent over time");
    }

    #[test]
    fn test_different_but_equivalent_dates_different_hash() {
        let now = Utc::now();
        let thirty_days_later = now + Duration::days(30);
        let exp1 = ExpirationDate::Days(Positive::new(30.0).unwrap());
        let exp2 = ExpirationDate::DateTime(thirty_days_later);
        // Different variants must hash differently even when they may equal.
        assert_ne!(calculate_hash(&exp1), calculate_hash(&exp2));
    }
}

mod tests_comparisons {
    use super::*;
    use crate::EPSILON;
    use positive::pos_or_panic;
    use rust_decimal_macros::dec;
    use std::cmp::Ordering;

    #[test]
    fn test_partial_eq_days_variants_equal() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_days_variants_within_epsilon() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 =
            ExpirationDate::Days(Positive::new_decimal(dec!(30.0) + EPSILON / dec!(2.0)).unwrap());
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_days_variants_outside_epsilon() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.1));
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_partial_eq_datetime_variants_equal() {
        let datetime = Utc.with_ymd_and_hms(2024, 12, 15, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime);
        let date2 = ExpirationDate::DateTime(datetime);
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_datetime_variants_different() {
        let datetime1 = Utc.with_ymd_and_hms(2027, 12, 15, 16, 0, 0).unwrap();
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 16, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime1);
        let date2 = ExpirationDate::DateTime(datetime2);
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_partial_eq_mixed_variants_with_zero_fallback() {
        let days_date = ExpirationDate::Days(Positive::ZERO);
        // A past DateTime should clamp to ZERO days.
        let past_datetime = Utc::now() - Duration::days(10);
        let datetime_date = ExpirationDate::DateTime(past_datetime);
        assert_eq!(days_date, datetime_date);
    }

    #[test]
    fn test_eq_trait_consistency() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date3 = ExpirationDate::Days(pos_or_panic!(30.0));

        // Reflexive
        assert_eq!(date1, date1);
        // Symmetric
        assert_eq!(date1, date2);
        assert_eq!(date2, date1);
        // Transitive
        assert_eq!(date1, date2);
        assert_eq!(date2, date3);
        assert_eq!(date1, date3);
    }

    #[test]
    fn test_partial_ord_returns_some() {
        let date1 = ExpirationDate::Days(pos_or_panic!(15.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        let result = date1.partial_cmp(&date2);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Ordering::Less);
    }

    #[test]
    fn test_ord_days_variants_less() {
        let date1 = ExpirationDate::Days(pos_or_panic!(15.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1.cmp(&date2), Ordering::Less);
    }

    #[test]
    fn test_ord_days_variants_greater() {
        let date1 = ExpirationDate::Days(pos_or_panic!(45.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1.cmp(&date2), Ordering::Greater);

        let date1 = ExpirationDate::Days(pos_or_panic!(45.0));
        let datetime2 = Utc.with_ymd_and_hms(2099, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);
        assert_eq!(date1.cmp(&date2), Ordering::Less);

        let datetime1 = Utc.with_ymd_and_hms(2098, 12, 20, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime1);
        let datetime2 = Utc.with_ymd_and_hms(2099, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);
        assert_eq!(date1.cmp(&date2), Ordering::Less);

        let date1 = ExpirationDate::Days(pos_or_panic!(100_000.0));
        let datetime2 = Utc.with_ymd_and_hms(2099, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);
        assert_eq!(date1.cmp(&date2), Ordering::Greater);
    }

    #[test]
    fn test_ord_days_variants_equal() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1.cmp(&date2), Ordering::Equal);
    }

    #[test]
    fn test_ord_datetime_variants() {
        let datetime1 = Utc.with_ymd_and_hms(2099, 12, 15, 16, 0, 0).unwrap();
        let datetime2 = Utc.with_ymd_and_hms(2099, 12, 20, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime1);
        let date2 = ExpirationDate::DateTime(datetime2);
        let result = date1.cmp(&date2);
        assert!(result != Ordering::Equal);
    }

    #[test]
    fn test_ord_mixed_variants() {
        let days_date = ExpirationDate::Days(pos_or_panic!(20.0));
        let future_datetime = Utc::now() + Duration::days(30);
        let datetime_date = ExpirationDate::DateTime(future_datetime);
        let result = days_date.cmp(&datetime_date);
        assert!(result != Ordering::Equal);
        assert_eq!(result, Ordering::Less);
    }

    #[test]
    fn test_ord_with_zero_fallback() {
        let date1 = ExpirationDate::Days(Positive::ZERO);
        let date2 = ExpirationDate::Days(pos_or_panic!(10.0));
        assert_eq!(date1.cmp(&date2), Ordering::Less);
        assert_eq!(date2.cmp(&date1), Ordering::Greater);
    }

    #[test]
    fn test_ord_consistency_with_partial_ord() {
        let date1 = ExpirationDate::Days(pos_or_panic!(25.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(35.0));
        let ord_result = date1.cmp(&date2);
        let partial_ord_result = date1.partial_cmp(&date2);
        assert_eq!(Some(ord_result), partial_ord_result);
    }

    #[test]
    fn test_ord_transitivity() {
        let date1 = ExpirationDate::Days(pos_or_panic!(10.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(20.0));
        let date3 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1.cmp(&date2), Ordering::Less);
        assert_eq!(date2.cmp(&date3), Ordering::Less);
        assert_eq!(date1.cmp(&date3), Ordering::Less);
    }

    #[test]
    fn test_ord_antisymmetry() {
        let date1 = ExpirationDate::Days(pos_or_panic!(25.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(25.0));
        assert!(date1.cmp(&date2) <= Ordering::Equal);
        assert!(date2.cmp(&date1) <= Ordering::Equal);
        assert_eq!(date1.cmp(&date2), Ordering::Equal);
    }

    #[test]
    fn test_ord_reflexivity() {
        let date = ExpirationDate::Days(pos_or_panic!(25.0));
        assert_eq!(date.cmp(&date), Ordering::Equal);

        let datetime = Utc.with_ymd_and_hms(2024, 12, 15, 16, 0, 0).unwrap();
        let datetime_date = ExpirationDate::DateTime(datetime);
        assert_eq!(datetime_date.cmp(&datetime_date), Ordering::Equal);
    }

    #[test]
    fn test_sorting_expiration_dates() {
        let mut dates = vec![
            ExpirationDate::Days(pos_or_panic!(45.0)),
            ExpirationDate::Days(pos_or_panic!(15.0)),
            ExpirationDate::Days(pos_or_panic!(30.0)),
            ExpirationDate::Days(pos_or_panic!(5.0)),
        ];
        dates.sort();

        let expected = vec![
            ExpirationDate::Days(pos_or_panic!(5.0)),
            ExpirationDate::Days(pos_or_panic!(15.0)),
            ExpirationDate::Days(pos_or_panic!(30.0)),
            ExpirationDate::Days(pos_or_panic!(45.0)),
        ];
        assert_eq!(dates, expected);
    }

    #[test]
    fn test_partial_eq_edge_case_epsilon_boundary() {
        let base_value = Positive::HUNDRED;
        let date1 = ExpirationDate::Days(base_value);
        let date2 =
            ExpirationDate::Days(Positive::new_decimal(base_value.value() + EPSILON).unwrap());
        // Difference equals (not less than) EPSILON, so values must be unequal.
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_mixed_variant_comparison_edge_cases() {
        let zero_days = ExpirationDate::Days(Positive::ZERO);
        let very_old_datetime = Utc.with_ymd_and_hms(1990, 1, 1, 0, 0, 0).unwrap();
        let old_datetime_date = ExpirationDate::DateTime(very_old_datetime);
        // Both clamp to ZERO days and must compare equal.
        assert_eq!(zero_days, old_datetime_date);
    }
}
