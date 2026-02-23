//! Financial conventions for day counting and business days.

use crate::error::ExpirationDateError;
use chrono::{DateTime, Datelike, Utc};

/// Trait for financial day count conventions.
pub trait DayCount: Copy + Send + Sync {
    /// Returns the year fraction between start and end dates.
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ArithmeticOverflow] if the calculation fails.
    fn year_fraction(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError>;
    
    /// Returns the number of days between start and end dates.
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ArithmeticOverflow] if the calculation fails.
    fn day_count(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError>;
}

/// Actual/360 day count convention. Commonly used in money markets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Actual360;

impl DayCount for Actual360 {
    #[inline]
    fn year_fraction(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        Ok(self.day_count(start, end)? / 360.0)
    }

    #[inline]
    fn day_count(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        let duration = end.signed_duration_since(*start);
        Ok(duration.num_days() as f64)
    }
}

/// Actual/365 Fixed day count convention. Standard for many derivatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Actual365Fixed;

impl DayCount for Actual365Fixed {
    #[inline]
    fn year_fraction(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        Ok(self.day_count(start, end)? / 365.0)
    }

    #[inline]
    fn day_count(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        let duration = end.signed_duration_since(*start);
        Ok(duration.num_days() as f64)
    }
}

/// 30/360 US (NASD) day count convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Thirty360US;

impl DayCount for Thirty360US {
    #[inline]
    fn year_fraction(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        Ok(self.day_count(start, end)? / 360.0)
    }

    fn day_count(&self, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<f64, ExpirationDateError> {
        let d1 = start.day().min(30) as i64;
        let mut d2 = end.day() as i64;
        if d1 == 30 && d2 == 31 { d2 = 30; }
        
        let m1 = start.month() as i64;
        let m2 = end.month() as i64;
        let y1 = start.year() as i64;
        let y2 = end.year() as i64;
        
        let y_diff = y2.checked_sub(y1).ok_or(ExpirationDateError::ArithmeticOverflow)?;
        let m_diff = m2.checked_sub(m1).ok_or(ExpirationDateError::ArithmeticOverflow)?;
        let d_diff = d2.checked_sub(d1).ok_or(ExpirationDateError::ArithmeticOverflow)?;

        let term1 = y_diff.checked_mul(360).ok_or(ExpirationDateError::ArithmeticOverflow)?;
        let term2 = m_diff.checked_mul(30).ok_or(ExpirationDateError::ArithmeticOverflow)?;
        
        let total = term1.checked_add(term2)
            .and_then(|t| t.checked_add(d_diff))
            .ok_or(ExpirationDateError::ArithmeticOverflow)?;

        Ok(total as f64)
    }
}
