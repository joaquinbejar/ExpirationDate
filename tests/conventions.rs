//! Integration tests for the day-count conventions module and the
//! reference-datetime accessor used by relative Day calculations.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use chrono::{Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;
use expiration_date::conventions::{Actual360, DayCount, Thirty360US};
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
