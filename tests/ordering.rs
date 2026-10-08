//! Integration tests for the hand-written `PartialEq` / `Eq` / `PartialOrd`
//! / `Ord` impls on [`ExpirationDate`]: exact equality, ordering by the
//! resolved instant, and sort stability.

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

/// Equality is exact since 0.4.2: a tolerance is not transitive and cannot
/// agree with `Hash`, so two day counts half an `EPSILON` apart are distinct.
#[test]
fn test_partial_eq_days_variants_within_epsilon_are_distinct() {
    let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
    let date2 =
        ExpirationDate::Days(Positive::new_decimal(dec!(30.0) + EPSILON / dec!(2.0)).unwrap());
    assert_ne!(date1, date2);
    assert_eq!(date1.cmp(&date2), Ordering::Less);
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

/// A past `DateTime` is no longer clamped to zero days: it stays its own
/// instant, before `Days(0)` (now), and is never equal to a `Days`.
#[test]
fn test_past_datetime_is_not_clamped_to_zero_days() {
    let days_date = ExpirationDate::Days(Positive::ZERO);
    let past_datetime = Utc::now() - Duration::days(10);
    let datetime_date = ExpirationDate::DateTime(past_datetime);
    assert_ne!(days_date, datetime_date);
    assert_eq!(datetime_date.cmp(&days_date), Ordering::Less);
    assert_eq!(days_date.cmp(&datetime_date), Ordering::Greater);
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
    // 1990 is long before now, which is where `Days(0)` resolves.
    assert_ne!(zero_days, old_datetime_date);
    assert!(old_datetime_date < zero_days);
}

/// Two distinct expiries that have both passed were clamped to zero days and
/// compared equal before 0.4.2 (joaquinbejar/OptionStratLib#825).
#[test]
fn test_distinct_past_dates_are_unequal_and_ordered() {
    let earlier = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2020, 3, 20, 16, 0, 0).unwrap());
    let later = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2020, 6, 19, 16, 0, 0).unwrap());
    assert_ne!(earlier, later);
    assert_eq!(earlier.cmp(&later), Ordering::Less);
    assert_eq!(later.cmp(&earlier), Ordering::Greater);
}

/// The order of two future dates does not depend on when it is asked.
#[test]
fn test_future_dates_order_is_stable() {
    let near = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2098, 3, 20, 16, 0, 0).unwrap());
    let far = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2099, 6, 19, 16, 0, 0).unwrap());
    let first = near.cmp(&far);
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(first, Ordering::Less);
    assert_eq!(near.cmp(&far), first);
}

/// An ordered map keyed by expiry keeps every past expiry as its own entry.
#[test]
fn test_btreemap_keeps_two_past_expiries() {
    let mut chains = std::collections::BTreeMap::new();
    let march = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2020, 3, 20, 16, 0, 0).unwrap());
    let june = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2020, 6, 19, 16, 0, 0).unwrap());
    chains.insert(june, "june");
    chains.insert(march, "march");
    assert_eq!(chains.len(), 2);
    assert_eq!(
        chains.values().copied().collect::<Vec<_>>(),
        ["march", "june"]
    );
    assert_eq!(chains.get(&march), Some(&"march"));
}

/// With the base pinned, a `Days` and a `DateTime` that resolve to the same
/// instant are still unequal, and the tie goes to the `Days` both ways round.
#[test]
fn test_mixed_variant_tie_orders_days_first() {
    let base = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    ExpirationDate::set_reference_datetime(Some(base));
    let days = ExpirationDate::Days(pos_or_panic!(30.0));
    let same_instant = ExpirationDate::DateTime(base + Duration::days(30));
    let tie = (days.cmp(&same_instant), same_instant.cmp(&days));
    let later = ExpirationDate::DateTime(base + Duration::days(31));
    let before_later = days.cmp(&later);
    ExpirationDate::set_reference_datetime(None);

    assert_ne!(days, same_instant);
    assert_eq!(tie, (Ordering::Less, Ordering::Greater));
    assert_eq!(before_later, Ordering::Less);
}

/// A day count past the last representable `DateTime` sorts after every
/// `DateTime` rather than failing.
#[test]
fn test_days_beyond_datetime_range_sort_last() {
    let far = ExpirationDate::Days(Positive::MAX);
    let date = ExpirationDate::DateTime(Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap());
    assert_eq!(far.cmp(&date), Ordering::Greater);
    assert_eq!(date.cmp(&far), Ordering::Less);
}
