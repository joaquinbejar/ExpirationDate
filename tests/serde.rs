//! Integration tests covering serde serialization / deserialization and
//! wire-shape contracts for [`ExpirationDate`].

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{TimeZone, Utc};
use expiration_date::ExpirationDate;
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
