use crate::ExpirationDate;
use crate::conventions::{DayCount, Actual360, Thirty360US};
use chrono::{Duration, TimeZone, Utc};
use positive::Positive;
use positive::constants::DAYS_IN_A_YEAR;
use std::error::Error;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

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
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().ok_or("date error")?;
    let end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().ok_or("date error")?;
    let conv = Thirty360US;
    let days = conv.day_count(&start, &end)?;
    assert!((days - 360.0).abs() < f64::EPSILON);
    Ok(())
}

#[test]
fn test_parsing_multi_formats() -> Result<(), Box<dyn Error>> {
    let formats = [
        ("2025-12-31", 2025, 12, 31),
        ("31-12-2025", 2025, 12, 31),
        ("20251231", 2025, 12, 31),
    ];
    for (s, y, m, d) in formats {
        let exp = ExpirationDate::from_string(s)?;
        match exp {
            ExpirationDate::DateTime(dt) => {
                use chrono::Datelike;
                assert_eq!(dt.year(), y);
                assert_eq!(dt.month(), m);
                assert_eq!(dt.day(), d);
            },
            _ => return Err(format!("Failed format {}", s).into()),
        }
    }
    Ok(())
}

#[test]
fn test_hash_consistency() -> Result<(), Box<dyn Error>> {
    let d1 = ExpirationDate::Days(Positive::TEN);
    let d2 = ExpirationDate::Days(Positive::TEN);
    assert_eq!(calculate_hash(&d1), calculate_hash(&d2));
    Ok(())
}

#[test]
fn test_serialization_roundtrip() -> Result<(), Box<dyn Error>> {
    let original = ExpirationDate::Days(Positive::TEN);
    let json = serde_json::to_string(&original)?;
    let deserialized: ExpirationDate = serde_json::from_str(&json)?;
    assert_eq!(original, deserialized);
    Ok(())
}

#[test]
fn test_reference_datetime() -> Result<(), Box<dyn Error>> {
    let ref_dt = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).single().ok_or("date error")?;
    ExpirationDate::set_reference_datetime(Some(ref_dt));
    let exp = ExpirationDate::Days(Positive::ONE);
    let resolved = exp.get_date()?;
    assert_eq!(resolved, ref_dt + Duration::days(1));
    ExpirationDate::set_reference_datetime(None);
    Ok(())
}
