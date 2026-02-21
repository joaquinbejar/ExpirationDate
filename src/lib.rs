//! # ExpirationDate
//!
//! A high-performance financial instrument expiration date management library.
//!
//! Designed for low-latency quantitative finance systems, this crate provides
//! type-safe mechanisms to handle contract expirations and time-to-maturity
//! calculations using standard financial conventions.
//!
//! # Safety
//! This library contains zero unsafe code and maintains a strict "no-panic" policy in production.

pub mod error;
pub mod prelude;
pub mod conventions;

use crate::error::ExpirationDateError;
use crate::conventions::{DayCount, Actual365Fixed};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use positive::Positive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Small decimal value used for high-precision equality comparisons.
pub const EPSILON: Decimal = dec!(1e-16);

/// Represents the expiration of a financial instrument.
///
/// Supports relative (Days) and absolute (DateTime) specifications.
/// Implements [Copy] for stack-allocation efficiency in hot loops.
#[derive(Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[must_use = "Financial results must be used to ensure correct pricing."]
pub enum ExpirationDate {
    /// Relative expiration in positive fractional days.
    Days(Positive),
    /// Absolute expiration point in UTC.
    DateTime(DateTime<Utc>),
}

impl PartialEq for ExpirationDate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.get_days(), other.get_days()) {
            (Ok(s), Ok(o)) => (s.value() - o.value()).abs() < EPSILON,
            _ => false,
        }
    }
}

impl Eq for ExpirationDate {}

impl PartialOrd for ExpirationDate {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpirationDate {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.get_days(), other.get_days()) {
            (Ok(self_days), Ok(other_days)) => self_days.cmp(&other_days),
            (Err(_), _) => Ordering::Less,
            (_, Err(_)) => Ordering::Greater,
        }
    }
}

impl ExpirationDate {
    /// Calculates the time to expiration in years using a specific day count convention.
    ///
    /// # Invariants
    /// - If the target date is in the past relative to [Utc::now], returns [Positive::ZERO].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation or conversion fails.
    ///
    /// # Examples
    /// ```
    /// use expiration_date::ExpirationDate;
    /// use expiration_date::conventions::Actual360;
    /// use positive::Positive;
    /// 
    /// let exp = ExpirationDate::Days(Positive::HUNDRED);
    /// let years = exp.get_years_with_convention(Actual360).unwrap();
    /// ```
    #[must_use]
    #[inline]
    pub fn get_years_with_convention<C: DayCount>(&self, convention: C) -> Result<Positive, ExpirationDateError> {
        let now = Utc::now();
        let target_date = self.get_date()?;
        
        if target_date <= now {
            return Ok(Positive::ZERO);
        }

        let year_fraction = convention.year_fraction(&now, &target_date)?;
        Positive::new(year_fraction).map_err(Into::into)
    }

    /// Calculates years using standard Actual/365 Fixed convention.
    #[must_use]
    #[inline]
    pub fn get_years(&self) -> Result<Positive, ExpirationDateError> {
        self.get_years_with_convention(Actual365Fixed)
    }

    /// Returns the number of fractional days until expiration.
    ///
    /// # Errors
    /// Returns error if current system time cannot be compared with expiration.
    #[must_use]
    #[inline]
    pub fn get_days(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => Ok(*days),
            Self::DateTime(dt) => {
                let now = Utc::now();
                let duration = dt.signed_duration_since(now);
                let num_days = duration.num_seconds() as f64 / 86400.0;
                
                if num_days <= 0.0 {
                    return Ok(Positive::ZERO);
                }
                Positive::new(num_days).map_err(Into::into)
            }
        }
    }

    /// Resolves expiration to an absolute [DateTime<Utc>].
    #[must_use]
    #[inline]
    pub fn get_date(&self) -> Result<DateTime<Utc>, ExpirationDateError> {
        self.get_date_with_options(false)
    }

    /// Resolves datetime with advanced base-time options.
    ///
    /// # Arguments
    /// - `use_fixed_time`: If true, base is today at 18:30 UTC for relative Days.
    #[must_use]
    pub fn get_date_with_options(&self, use_fixed_time: bool) -> Result<DateTime<Utc>, ExpirationDateError> {
        match self {
            Self::Days(days) => {
                let base = if use_fixed_time {
                    Utc::now().date_naive().and_hms_opt(18, 30, 0)
                        .ok_or_else(|| ExpirationDateError::InvalidDateTime("Time construction error".into()))?
                } else {
                    Utc::now().naive_utc()
                };
                
                let base_dt = DateTime::<Utc>::from_naive_utc_and_offset(base, Utc);
                let days_i64 = (*days).to_i64();
                Ok(base_dt + Duration::days(days_i64))
            }
            Self::DateTime(dt) => Ok(*dt),
        }
    }

    /// Parse expiration from string using HFT-friendly formats (YYYYMMDD).
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ParseError] if format is unrecognized.
    #[must_use]
    pub fn from_string(s: &str) -> Result<Self, ExpirationDateError> {
        // Try direct day count
        if let Ok(days) = s.parse::<Positive>() {
            return Ok(Self::Days(days));
        }

        // Optimized YYYYMMDD check
        if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
            let year = s[0..4].parse::<i32>()?;
            let month = s[4..6].parse::<u32>()?;
            let day = s[6..8].parse::<u32>()?;

            if let Some(nd) = NaiveDate::from_ymd_opt(year, month, day) {
                if let Some(ndt) = nd.and_hms_opt(23, 59, 59) {
                    return Ok(Self::DateTime(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)));
                }
            }
        }

        // Standard ISO-8601
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            let dt = date.and_hms_opt(18, 30, 0)
                .ok_or_else(|| ExpirationDateError::InvalidDateTime("ISO parse failed".into()))?;
            return Ok(Self::DateTime(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)));
        }

        Err(ExpirationDateError::ParseError(s.to_string()))
    }
}

impl Default for ExpirationDate {
    #[inline]
    fn default() -> Self {
        Self::Days(Positive::THREE_HUNDRED_SIXTY_FIVE)
    }
}

impl fmt::Display for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_date() {
            Ok(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S UTC")),
            Err(_) => write!(f, "Invalid Expiration"),
        }
    }
}

impl Serialize for ExpirationDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeMap;
        match self {
            Self::Days(days) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("days", &days.to_f64())?;
                map.end()
            },
            Self::DateTime(dt) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("datetime", &dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ExpirationDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use serde::de::{Visitor, MapAccess};

        struct ExpirationVisitor;
        impl<'de> Visitor<'de> for ExpirationVisitor {
            type Value = ExpirationDate;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("expiration object") }
            fn visit_map<V>(self, mut map: V) -> Result<ExpirationDate, V::Error> where V: MapAccess<'de> {
                let mut days = None;
                let mut dt = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "days" => days = Some(map.next_value::<f64>()?),
                        "datetime" => dt = Some(map.next_value::<String>()?),
                        _ => { let _ = map.next_value::<serde::de::IgnoredAny>()?; }
                    }
                }
                match (days, dt) {
                    (Some(d), _) => Ok(ExpirationDate::Days(Positive::new(d).map_err(serde::de::Error::custom)?)),
                    (_, Some(t)) => {
                        let parsed = DateTime::parse_from_rfc3339(&t)
                            .map_err(serde::de::Error::custom)?
                            .with_timezone(&Utc);
                        Ok(ExpirationDate::DateTime(parsed))
                    }
                    _ => Err(serde::de::Error::missing_field("days or datetime")),
                }
            }
        }
        deserializer.deserialize_map(ExpirationVisitor)
    }
}
