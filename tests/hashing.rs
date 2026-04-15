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
