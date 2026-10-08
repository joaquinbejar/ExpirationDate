//! Integration tests for the hand-written `Hash` impl on [`ExpirationDate`].

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;
use positive::Positive;
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

/// `a == b` implies `hash(a) == hash(b)`, including for equal day counts
/// written at different scales and for past instants.
#[test]
fn test_eq_implies_equal_hash() {
    use rust_decimal_macros::dec;

    let scaled = ExpirationDate::Days(Positive::new_decimal(dec!(30.0)).unwrap());
    let plain = ExpirationDate::Days(Positive::new_decimal(dec!(30)).unwrap());
    assert_eq!(scaled, plain);
    assert_eq!(calculate_hash(&scaled), calculate_hash(&plain));

    let past = Utc.with_ymd_and_hms(2020, 3, 20, 16, 0, 0).unwrap();
    let a = ExpirationDate::DateTime(past);
    let b = ExpirationDate::DateTime(past);
    assert_eq!(a, b);
    assert_eq!(calculate_hash(&a), calculate_hash(&b));
}

/// Two past expiries are distinct keys in a hash set.
#[test]
fn test_hash_set_keeps_two_past_expiries() {
    let mut set = std::collections::HashSet::new();
    set.insert(ExpirationDate::DateTime(
        Utc.with_ymd_and_hms(2020, 3, 20, 16, 0, 0).unwrap(),
    ));
    set.insert(ExpirationDate::DateTime(
        Utc.with_ymd_and_hms(2020, 6, 19, 16, 0, 0).unwrap(),
    ));
    assert_eq!(set.len(), 2);
}

proptest::proptest! {
    /// Over arbitrary pairs of either variant, `==` agrees with `cmp ==
    /// Equal`, and equal values hash equal.
    #[test]
    fn prop_eq_ord_and_hash_agree(
        a_days in proptest::option::of(0u32..100_000u32),
        a_secs in -2_000_000_000i64..4_000_000_000i64,
        b_days in proptest::option::of(0u32..100_000u32),
        b_secs in -2_000_000_000i64..4_000_000_000i64,
    ) {
        let make = |days: Option<u32>, secs: i64| match days {
            Some(d) => ExpirationDate::Days(Positive::new(f64::from(d) / 4.0).unwrap()),
            None => ExpirationDate::DateTime(Utc.timestamp_opt(secs, 0).unwrap()),
        };
        let a = make(a_days, a_secs);
        let b = make(b_days, b_secs);
        proptest::prop_assert_eq!(a == b, a.cmp(&b) == std::cmp::Ordering::Equal);
        proptest::prop_assert_eq!(a.cmp(&b), b.cmp(&a).reverse());
        if a == b {
            proptest::prop_assert_eq!(calculate_hash(&a), calculate_hash(&b));
        }
    }
}
