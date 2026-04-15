//! # ExpirationDate
//!
//! A professional high-performance financial instrument expiration date management library.

/// Error handling module for expiration date operations.
pub mod error;
/// Prelude module for common traits and types.
pub mod prelude;
/// Financial day count conventions module.
pub mod conventions;
#[cfg(test)]
mod tests;

use crate::error::ExpirationDateError;
use crate::conventions::{DayCount, Actual365Fixed};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use positive::Positive;
use positive::constants::DAYS_IN_A_YEAR;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Small decimal value used for high-precision equality comparisons.
pub const EPSILON: Decimal = dec!(1e-16);

/// Represents the expiration of a financial instrument.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[must_use = "Financial expiration results must be used for correct pricing calculations."]
pub enum ExpirationDate {
    /// Relative expiration in positive fractional days.
    Days(Positive),
    /// Absolute expiration point in UTC.
    DateTime(DateTime<Utc>),
}

impl Hash for ExpirationDate {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Days(d) => {
                state.write_u8(0);
                d.hash(state);
            }
            Self::DateTime(dt) => {
                state.write_u8(1);
                dt.timestamp().hash(state);
                dt.timestamp_subsec_nanos().hash(state);
            }
        }
    }
}

impl PartialEq for ExpirationDate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.get_days(), other.get_days()) {
            (Ok(s), Ok(o)) => (s.to_dec() - o.to_dec()).abs() < EPSILON,
            // If day conversion fails for either side, avoid silently treating it as zero.
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
    thread_local! {
        static REFERENCE_DATETIME: std::cell::RefCell<Option<DateTime<Utc>>> = const { std::cell::RefCell::new(None) };
    }

    /// Sets the reference datetime for Days variant calculations.
    pub fn set_reference_datetime(dt: Option<DateTime<Utc>>) {
        Self::REFERENCE_DATETIME.with(|cell| {
            *cell.borrow_mut() = dt;
        });
    }

    /// Gets the current reference datetime.
    #[must_use = "Retrieving the reference datetime has no effect if the result is ignored."]
    pub fn get_reference_datetime() -> Option<DateTime<Utc>> {
        Self::REFERENCE_DATETIME.with(|cell| *cell.borrow())
    }

    /// Calculates years using a specific day count convention.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    pub fn get_years_with_convention<C: DayCount>(&self, convention: C) -> Result<Positive, ExpirationDateError> {
        let now = Utc::now();
        let target_date = self.get_date_with_base(now)?;
        if target_date <= now { return Ok(Positive::ZERO); }
        let fraction = convention.year_fraction(&now, &target_date)?;
        Positive::new(fraction).map_err(Into::into)
    }

    /// Calculates years until expiration using standard Actual/365 Fixed.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    #[inline]
    pub fn get_years(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => {
                let years = days.to_f64() / DAYS_IN_A_YEAR.to_f64();
                Positive::new(years).map_err(Into::into)
            },
            Self::DateTime(_) => self.get_years_with_convention(Actual365Fixed)
        }
    }

    /// Returns the number of fractional days until expiration.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if conversion to Positive fails.
    #[must_use = "Calculated days must be used to ensure valid financial models."]
    #[inline]
    pub fn get_days(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => Ok(*days),
            Self::DateTime(dt) => {
                let now = Utc::now();
                let duration = dt.signed_duration_since(now);
                let num_days = duration.num_seconds() as f64 / 86400.0;
                if num_days <= 0.0 { return Ok(Positive::ZERO); }
                Positive::new(num_days).map_err(Into::into)
            }
        }
    }

    /// Resolves expiration to an absolute [DateTime<Utc>].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if construction fails.
    #[must_use = "Resolved date must be used for settlement or further calculations."]
    #[inline]
    pub fn get_date(&self) -> Result<DateTime<Utc>, ExpirationDateError> {
        self.get_date_with_options(false)
    }

    fn get_date_with_base(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>, ExpirationDateError> {
        match self {
            Self::Days(days) => {
                let base = Self::get_reference_datetime().unwrap_or(now);
                Ok(base + Duration::days((*days).to_i64()))
            }
            Self::DateTime(dt) => Ok(*dt),
        }
    }

    /// Resolves datetime with advanced base-time options.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if the construction fails.
    #[must_use = "Resolved date must be used for settlement or further calculations."]
    pub fn get_date_with_options(&self, use_fixed_time: bool) -> Result<DateTime<Utc>, ExpirationDateError> {
        if use_fixed_time {
            let today = Utc::now().date_naive();
            let fixed = today.and_hms_opt(18, 30, 0)
                .ok_or_else(|| ExpirationDateError::InvalidDateTime("Fixed time error".into()))?;
            let base_dt = DateTime::<Utc>::from_naive_utc_and_offset(fixed, Utc);
            if let Self::Days(days) = self {
                return Ok(base_dt + Duration::days((*days).to_i64()));
            }
        }
        self.get_date_with_base(Utc::now())
    }

    /// Returns the expiration date as a formatted YYYY-MM-DD string.
    ///
    /// # Errors
    /// Returns error if the date cannot be resolved.
    #[must_use = "Formatted string should be consumed for display or reporting."]
    pub fn get_date_string(&self) -> Result<String, ExpirationDateError> {
        let date = self.get_date_with_options(true)?;
        Ok(date.format("%Y-%m-%d").to_string())
    }

    /// Parse expiration from string using multiple formats.
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ParseError] if format matches no known pattern.
    #[must_use = "Parsed expiration result must be used."]
    pub fn from_string(s: &str) -> Result<Self, ExpirationDateError> {
        let formats = ["%Y-%m-%d", "%d-%m-%Y", "%d %b %Y", "%d-%b-%Y", "%Y%m%d"];
        for fmt in formats {
            if let Ok(date) = NaiveDate::parse_from_str(s, fmt) {
                let dt = date.and_hms_opt(18, 30, 0).ok_or(ExpirationDateError::ArithmeticOverflow)?;
                return Ok(Self::DateTime(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)));
            }
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(Self::DateTime(dt.with_timezone(&Utc)));
        }
        if let Ok(days) = s.parse::<Positive>() {
            return Ok(Self::Days(days));
        }
        Err(ExpirationDateError::ParseError(s.to_string()))
    }

    /// Parses string and converts result to Days variant.
    ///
    /// # Errors
    /// Returns error if parsing or conversion fails.
    #[must_use = "Parsed days result must be used."]
    pub fn from_string_to_days(s: &str) -> Result<Self, ExpirationDateError> {
        let exp = Self::from_string(s)?;
        Ok(Self::Days(exp.get_days()?))
    }
}

impl Default for ExpirationDate {
    #[inline]
    fn default() -> Self {
        Self::Days(DAYS_IN_A_YEAR)
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
        let mut state = serializer.serialize_map(Some(1))?;
        match self {
            ExpirationDate::Days(days) => state.serialize_entry("days", &days.to_f64())?,
            ExpirationDate::DateTime(dt) => state.serialize_entry("datetime", &dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())?,
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ExpirationDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use serde::de::{Visitor, MapAccess};
        struct ExVisitor;
        impl<'de> Visitor<'de> for ExVisitor {
            type Value = ExpirationDate;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("struct ExpirationDate") }
            fn visit_map<V>(self, mut map: V) -> Result<ExpirationDate, V::Error> where V: MapAccess<'de> {
                let mut d = None; let mut t = None;
                while let Some(k) = map.next_key::<String>()? {
                    match k.as_str() {
                        "days" => d = Some(map.next_value::<f64>()?),
                        "datetime" => t = Some(map.next_value::<String>()?),
                        _ => { return Err(serde::de::Error::unknown_field(&k, &["days", "datetime"])); }
                    }
                }
                match (d, t) {
                    (Some(v), _) => Ok(ExpirationDate::Days(Positive::new(v).map_err(serde::de::Error::custom)?)),
                    (_, Some(v)) => Ok(ExpirationDate::DateTime(DateTime::parse_from_rfc3339(&v).map_err(serde::de::Error::custom)?.with_timezone(&Utc))),
                    _ => Err(serde::de::Error::missing_field("days or datetime")),
                }
            }
        }
        deserializer.deserialize_struct("ExpirationDate", &["days", "datetime"], ExVisitor)
    }
}
