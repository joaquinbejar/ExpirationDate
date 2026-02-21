use crate::ExpirationDate;
use crate::conventions::{DayCount, Actual360, Thirty360US};
use chrono::{Duration, TimeZone, Utc, Datelike};
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
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().ok_or("Invalid start date")?;
    let end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().ok_or("Invalid end date")?;
    
    let conv = Thirty360US;
    let days = conv.day_count(&start, &end)?;
    assert!((days - 360.0).abs() < f64::EPSILON);
    Ok(())
}

#[test]
fn test_parsing_hft_format() -> Result<(), Box<dyn Error>> {
    let s = "20251231";
    let exp = ExpirationDate::from_string(s)?;
    match exp {
        ExpirationDate::DateTime(dt) => {
            assert_eq!(dt.year(), 2025);
            assert_eq!(dt.month(), 12);
            assert_eq!(dt.day(), 31);
        },
        _ => return Err("Expected DateTime variant".into()),
    }
    Ok(())
}

#[test]
fn test_past_date_returns_zero() -> Result<(), Box<dyn Error>> {
    let past = Utc::now() - Duration::days(10);
    let exp = ExpirationDate::DateTime(past);
    assert_eq!(exp.get_years()?, Positive::ZERO);
    assert_eq!(exp.get_days()?, Positive::ZERO);
    Ok(())
}
