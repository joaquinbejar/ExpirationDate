//! Integration tests for the hand-written `PartialEq` / `Eq` / `PartialOrd`
//! / `Ord` impls on [`ExpirationDate`], including EPSILON semantics and
//! sort stability.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Duration, TimeZone, Utc};
use expiration_date::{EPSILON, ExpirationDate};
use positive::{Positive, pos_or_panic};
use rust_decimal_macros::dec;
use std::cmp::Ordering;

#[test]
fn test_comparisons_and_sorting() {
    let d1 = ExpirationDate::Days(Positive::ONE);
    let d2 = ExpirationDate::Days(Positive::TEN);
    let mut list = [d2, d1];
    list.sort();
    assert_eq!(list.first(), Some(&d1));
}

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
    let date2 = ExpirationDate::Days(Positive::new_decimal(base_value.value() + EPSILON).unwrap());
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
