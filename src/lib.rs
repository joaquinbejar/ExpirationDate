//! # ExpirationDate
//!
//! A professional high-performance financial instrument expiration date management library.

/// Financial day count conventions module.
pub mod conventions;
/// Error handling module for expiration date operations.
pub mod error;
/// Prelude module for common traits and types.
pub mod prelude;
#[cfg(test)]
mod tests;

use crate::conventions::{Actual365Fixed, DayCount};
use crate::error::ExpirationDateError;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
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
            // Keep a total order even on conversion errors, without masking them as ZERO.
            (Err(self_err), Err(other_err)) => self_err.to_string().cmp(&other_err.to_string()),
            (Err(_), Ok(_)) => Ordering::Less,
            (Ok(_), Err(_)) => Ordering::Greater,
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
    pub fn get_years_with_convention<C: DayCount>(
        &self,
        convention: C,
    ) -> Result<Positive, ExpirationDateError> {
        let now = Utc::now();
        let target_date = self.get_date_with_base(now)?;
        if target_date <= now {
            return Ok(Positive::ZERO);
        }
        let fraction = convention.year_fraction(&now, &target_date)?;
        Positive::new(fraction).map_err(Into::into)
    }

    /// Calculates years until expiration using standard Actual/365 Fixed.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Duration, Utc};
    /// use positive::{assert_pos_relative_eq, pos_or_panic, Positive};
    /// use expiration_date::ExpirationDate;
    ///
    /// let days = pos_or_panic!(365.0);
    /// let expiration_date_days = ExpirationDate::Days(days);
    /// let years = expiration_date_days.get_years().unwrap();
    /// assert_pos_relative_eq!(years, Positive::ONE, pos_or_panic!(0.001));
    ///
    /// let datetime = Utc::now() + Duration::days(365);
    /// let expiration_date_datetime = ExpirationDate::DateTime(datetime);
    /// let years = expiration_date_datetime.get_years().unwrap();
    /// assert_pos_relative_eq!(years, Positive::ONE, pos_or_panic!(0.01));
    /// ```
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    #[inline]
    pub fn get_years(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => {
                let years = days.to_f64() / DAYS_IN_A_YEAR.to_f64();
                Positive::new(years).map_err(Into::into)
            }
            Self::DateTime(_) => self.get_years_with_convention(Actual365Fixed),
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
                // Store the original datetime as reference so callers who later
                // re-derive a `Days` variant can format it consistently.
                Self::set_reference_datetime(Some(*dt));

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

    /// Resolves expiration to an absolute [`DateTime<Utc>`].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if construction fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Duration, Utc};
    /// use positive::pos_or_panic;
    /// use expiration_date::ExpirationDate;
    ///
    /// // Clear any stored reference datetime from previous tests so the
    /// // relative calculation uses `Utc::now()` as its base.
    /// ExpirationDate::set_reference_datetime(None);
    ///
    /// let days = pos_or_panic!(30.0);
    /// let expiration_date_days = ExpirationDate::Days(days);
    /// let future_date = Utc::now() + Duration::days(30);
    /// let calculated_date = expiration_date_days.get_date().unwrap();
    /// assert_eq!(calculated_date.date_naive(), future_date.date_naive());
    ///
    /// let datetime = Utc::now() + Duration::days(365);
    /// let expiration_date_datetime = ExpirationDate::DateTime(datetime);
    /// let stored_date = expiration_date_datetime.get_date().unwrap();
    /// assert_eq!(stored_date, datetime);
    /// ```
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
    pub fn get_date_with_options(
        &self,
        use_fixed_time: bool,
    ) -> Result<DateTime<Utc>, ExpirationDateError> {
        if use_fixed_time {
            let today = Utc::now().date_naive();
            let fixed = today
                .and_hms_opt(18, 30, 0)
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
    ///
    /// # Examples
    ///
    /// ```
    /// use positive::pos_or_panic;
    /// use expiration_date::ExpirationDate;
    ///
    /// let days = pos_or_panic!(30.0);
    /// let expiration_date = ExpirationDate::Days(days);
    /// let date_string = expiration_date.get_date_string().unwrap();
    /// assert!(date_string.len() == 10); // YYYY-MM-DD format
    /// ```
    #[must_use = "Formatted string should be consumed for display or reporting."]
    pub fn get_date_string(&self) -> Result<String, ExpirationDateError> {
        let date = self.get_date_with_options(true)?;
        Ok(date.format("%Y-%m-%d").to_string())
    }

    /// Parse expiration from string using multiple formats.
    ///
    /// Supports the following input shapes:
    /// 1. **Positive number of days:** e.g., `"30.0"`
    /// 2. **ISO date:** e.g., `"2025-01-01"`
    /// 3. **Date with time and timezone:** e.g., `"2025-01-01 12:00:00 UTC"`
    /// 4. **RFC3339-like with time:** e.g., `"2025-05-23T15:29"` or
    ///    `"2025-05-23T15:29:00Z"`
    /// 5. **Numeric date (YYYYMMDD):** e.g., `"20250101"`
    /// 6. **Common date formats:** e.g., `"01-01-2025"`, `"30 jan 2025"`,
    ///    `"30-jan-2025"`, `"30 january 2025"`, `"30-january-2025"`
    ///
    /// Month names are matched case-insensitively.
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ParseError] if format matches no known pattern,
    /// or [ExpirationDateError::InvalidDateTime] if the derived time components are
    /// out of range.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use expiration_date::ExpirationDate;
    /// use positive::pos_or_panic;
    ///
    /// let exp_days = ExpirationDate::from_string("365").unwrap();
    /// assert_eq!(exp_days, ExpirationDate::Days(pos_or_panic!(365.0)));
    ///
    /// let exp_date = ExpirationDate::from_string("2025-12-31").unwrap();
    /// # let _ = exp_date;
    /// ```
    #[must_use = "Parsed expiration result must be used."]
    pub fn from_string(s: &str) -> Result<Self, ExpirationDateError> {
        // First try parsing as Positive (days)
        if let Ok(days) = s.parse::<Positive>() {
            return Ok(Self::Days(days));
        }

        // RFC3339 (e.g., "2025-01-01T00:00:00Z")
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let utc_dt = dt.with_timezone(&Utc);
            Self::set_reference_datetime(Some(utc_dt));
            return Ok(Self::DateTime(utc_dt));
        }

        // Date with time and timezone, e.g. "2025-01-01 12:00:00 UTC"
        if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %Z") {
            let utc_dt = dt.with_timezone(&Utc);
            Self::set_reference_datetime(Some(utc_dt));
            return Ok(Self::DateTime(utc_dt));
        }

        // Explicit " UTC" suffix handling (matches `Display` output)
        if s.contains(" UTC") && s.contains(':') {
            for format in ["%Y-%m-%d %H:%M:%S %Z", "%Y-%m-%d %H:%M:%S UTC"] {
                if let Ok(datetime) = DateTime::parse_from_str(s, format) {
                    let utc_dt = DateTime::from(datetime);
                    Self::set_reference_datetime(Some(utc_dt));
                    return Ok(Self::DateTime(utc_dt));
                }
            }

            // Manual fallback: strip " UTC" and parse the naive portion.
            let date_time_part = s.trim_end_matches(" UTC").trim();
            if let Ok(dt) = NaiveDateTime::parse_from_str(date_time_part, "%Y-%m-%d %H:%M:%S") {
                let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
                Self::set_reference_datetime(Some(utc_dt));
                return Ok(Self::DateTime(utc_dt));
            }
        }

        // RFC3339-like without seconds, e.g., "2025-05-23T15:29"
        if s.contains('T') && s.matches(':').count() == 1 {
            let datetime_str = format!("{s}:00Z");
            if let Ok(datetime) = DateTime::parse_from_rfc3339(&datetime_str) {
                let utc_dt = DateTime::from(datetime);
                Self::set_reference_datetime(Some(utc_dt));
                return Ok(Self::DateTime(utc_dt));
            }
        }

        // YYYYMMDD
        if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
            let year = s
                .get(0..4)
                .ok_or_else(|| ExpirationDateError::ParseError(s.to_string()))?
                .parse::<i32>()?;
            let month = s
                .get(4..6)
                .ok_or_else(|| ExpirationDateError::ParseError(s.to_string()))?
                .parse::<u32>()?;
            let day = s
                .get(6..8)
                .ok_or_else(|| ExpirationDateError::ParseError(s.to_string()))?
                .parse::<u32>()?;

            if let Some(naive_datetime) = NaiveDate::from_ymd_opt(year, month, day)
                .and_then(|date| date.and_hms_opt(23, 59, 59))
            {
                let datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc);
                Self::set_reference_datetime(Some(datetime));
                return Ok(Self::DateTime(datetime));
            }
        }

        // Common date-only formats. Lowercase input so month names match case-insensitively.
        let formats = [
            "%Y-%m-%d", // "2024-01-01"
            "%d-%m-%Y", // "01-01-2025"
            "%d %b %Y", // "30 jan 2025"
            "%d-%b-%Y", // "30-jan-2025"
            "%d %B %Y", // "30 january 2025"
            "%d-%B-%Y", // "30-january-2025"
        ];

        let lowered = s.to_lowercase();
        for format in formats {
            if let Ok(naive_date) = NaiveDate::parse_from_str(lowered.as_str(), format) {
                let naive_datetime = naive_date.and_hms_opt(18, 30, 0).ok_or_else(|| {
                    ExpirationDateError::InvalidDateTime(format!(
                        "invalid time conversion for date: {s}"
                    ))
                })?;
                let datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc);
                Self::set_reference_datetime(Some(datetime));
                return Ok(Self::DateTime(datetime));
            }
        }

        Err(ExpirationDateError::ParseError(format!(
            "failed to parse expirationdate from string: {s}"
        )))
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
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut state = serializer.serialize_map(Some(1))?;
        match self {
            Self::Days(days) => state.serialize_entry("days", &days.to_f64())?,
            Self::DateTime(dt) => {
                state.serialize_entry("datetime", &dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())?
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ExpirationDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        struct ExVisitor;
        impl<'de> Visitor<'de> for ExVisitor {
            type Value = ExpirationDate;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct ExpirationDate")
            }
            fn visit_map<V>(self, mut map: V) -> Result<ExpirationDate, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut d = None;
                let mut t = None;
                while let Some(k) = map.next_key::<String>()? {
                    match k.as_str() {
                        "days" => {
                            if d.is_some() {
                                return Err(serde::de::Error::duplicate_field("days"));
                            }
                            d = Some(map.next_value::<f64>()?);
                        }
                        "datetime" => {
                            if t.is_some() {
                                return Err(serde::de::Error::duplicate_field("datetime"));
                            }
                            t = Some(map.next_value::<String>()?);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(&k, &["days", "datetime"]));
                        }
                    }
                }
                match (d, t) {
                    (Some(v), _) => Ok(ExpirationDate::Days(
                        Positive::new(v).map_err(serde::de::Error::custom)?,
                    )),
                    (_, Some(v)) => Ok(ExpirationDate::DateTime(
                        DateTime::parse_from_rfc3339(&v)
                            .map_err(serde::de::Error::custom)?
                            .with_timezone(&Utc),
                    )),
                    _ => Err(serde::de::Error::missing_field("days or datetime")),
                }
            }
        }
        deserializer.deserialize_struct("ExpirationDate", &["days", "datetime"], ExVisitor)
    }
}
