//! # ExpirationDate
//!
//! A standalone crate for handling financial instrument expiration dates.
//!
//! This crate provides the `ExpirationDate` enum which supports two representations:
//! - **Days**: A positive number of days from the current date
//! - **DateTime**: An absolute point in time using UTC datetime
//!
//! ## Features
//!
//! - Parse expiration dates from multiple string formats (RFC3339, YYYYMMDD, DD-MM-YYYY, etc.)
//! - Convert between days-based and datetime-based representations
//! - Calculate time to expiration in years or days
//! - Full serde serialization/deserialization support
//! - Optional `utoipa` support for OpenAPI schema generation (enable `utoipa` feature)
//!
//! ## Usage
//!
//! ```rust
//! use expiration_date::ExpirationDate;
//! use positive::pos_or_panic;
//!
//! // Create from days
//! let exp = ExpirationDate::Days(pos_or_panic!(30.0));
//! let years = exp.get_years().unwrap();
//!
//! // Create from string
//! let exp = ExpirationDate::from_string("2025-12-31").unwrap();
//! ```

pub mod error;
pub mod prelude;

use crate::error::ExpirationDateError;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use positive::Positive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Small decimal value used for equality comparisons between expiration dates.
pub const EPSILON: Decimal = dec!(1e-16);

/// Represents the expiration of an option contract or financial instrument.
///
/// This enum allows for two different ways to specify when something expires:
/// - As a number of days from the current date
/// - As a specific date and time
///
/// `ExpirationDate` is used throughout options modeling systems to handle
/// time-based calculations such as time decay (theta) and option valuation.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[must_use]
pub enum ExpirationDate {
    /// Represents expiration as a positive number of days from the current date.
    /// This is typically used for relative time specifications.
    /// when converting between Days and DateTime variants.
    Days(Positive),

    /// Represents expiration as an absolute point in time using UTC datetime.
    /// This is used when a precise expiration moment is known.
    DateTime(DateTime<Utc>),
}

impl Hash for ExpirationDate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ExpirationDate::Days(days) => {
                0.hash(state); // Variant discriminant
                days.hash(state);
            }
            ExpirationDate::DateTime(datetime) => {
                1.hash(state); // Variant discriminant
                datetime.timestamp().hash(state);
                datetime.timestamp_subsec_nanos().hash(state);
            }
        }
    }
}

impl PartialEq for ExpirationDate {
    fn eq(&self, other: &Self) -> bool {
        match (self.get_days(), other.get_days()) {
            (Ok(s), Ok(o)) => (s.0 - o.0).abs() < EPSILON,
            // If day conversion fails for either side, avoid silently treating it as zero.
            _ => false,
        }
    }
}

impl Eq for ExpirationDate {}

impl PartialOrd for ExpirationDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpirationDate {
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
    /// Calculates the time to expiration in years.
    ///
    /// Returns the number of years until expiration as a `Positive` value.
    /// One year is defined as 365 days.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion to years results in an invalid value.
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
    /// assert_pos_relative_eq!(years, Positive::ONE, pos_or_panic!(0.001));
    /// ```
    #[must_use = "discarding expiration years can hide pricing and risk calculation bugs"]
    pub fn get_years(&self) -> Result<Positive, ExpirationDateError> {
        let days = self.get_days()?;
        let years = days.to_f64() / positive::constants::DAYS_IN_A_YEAR.to_f64();
        Positive::new(years).map_err(|e| ExpirationDateError::ConversionError {
            from_type: "f64".to_string(),
            to_type: "Positive".to_string(),
            reason: format!("failed to convert years: {}", e),
        })
    }

    /// Calculates the number of days until expiration.
    ///
    /// For the `Days` variant, returns the stored days value directly.
    /// For the `DateTime` variant, calculates the difference between the stored
    /// datetime and the current time in fractional days.
    ///
    /// If the expiration date is in the past, returns `Positive::ZERO`.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion to `Positive` fails.
    #[must_use = "discarding expiration days can hide pricing and risk calculation bugs"]
    pub fn get_days(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            ExpirationDate::Days(days) => Ok(*days),
            ExpirationDate::DateTime(datetime) => {
                // Store the original datetime as reference for future use
                Self::set_reference_datetime(Some(*datetime));

                let now = Utc::now();
                let duration = datetime.signed_duration_since(now);
                let num_days = duration.num_seconds() as f64 / (24.0 * 60.0 * 60.0);
                if num_days <= 0.0 {
                    return Ok(Positive::ZERO);
                }
                Positive::new(num_days).map_err(|e| ExpirationDateError::ConversionError {
                    from_type: "f64".to_string(),
                    to_type: "Positive".to_string(),
                    reason: format!("failed to convert days: {}", e),
                })
            }
        }
    }

    /// Returns the expiration date as a `DateTime<Utc>`.
    ///
    /// For the `Days` variant, calculates the date by adding the specified number
    /// of days to the current date and time.
    /// For the `DateTime` variant, returns the stored `DateTime<Utc>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the date calculation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Duration, Utc};
    /// use positive::pos_or_panic;
    /// use expiration_date::ExpirationDate;
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
    #[must_use = "discarding expiration datetime can hide scheduling and settlement bugs"]
    pub fn get_date(&self) -> Result<DateTime<Utc>, ExpirationDateError> {
        self.get_date_with_options(false)
    }

    // Thread-local storage to store reference datetime for Days variant
    thread_local! {
        static REFERENCE_DATETIME: std::cell::RefCell<Option<DateTime<Utc>>> = const { std::cell::RefCell::new(None) };
    }

    /// Retrieves the reference `DateTime` stored in thread-local storage.
    ///
    /// This function accesses a thread-local variable to retrieve the stored
    /// reference datetime. The state is specific to the calling thread.
    ///
    /// # Returns
    ///
    /// - `Some(DateTime<Utc>)` if a reference datetime is stored.
    /// - `None` if no reference datetime is set.
    #[must_use = "reference datetime retrieval has no side effects unless its value is consumed"]
    pub fn get_reference_datetime() -> Option<DateTime<Utc>> {
        let mut result = None;
        Self::REFERENCE_DATETIME.with(|cell| {
            result = *cell.borrow();
        });
        result
    }

    /// Sets the reference datetime in thread-local storage.
    ///
    /// # Arguments
    ///
    /// * `dt` - An `Option<DateTime<Utc>>` to store. Pass `None` to clear it.
    pub fn set_reference_datetime(dt: Option<DateTime<Utc>>) {
        Self::REFERENCE_DATETIME.with(|cell| {
            *cell.borrow_mut() = dt;
        });
    }

    /// Calculates and returns a `DateTime<Utc>` based on the specified options.
    ///
    /// # Arguments
    ///
    /// * `use_fixed_time` - If `true`, uses 18:30 UTC as the base time for the
    ///   `Days` variant. If `false`, uses the reference datetime or current time.
    ///
    /// # Returns
    ///
    /// - `Ok(DateTime<Utc>)` with the calculated expiration datetime.
    /// - `Err(ExpirationDateError)` if there is an invalid time conversion.
    ///
    /// # Behavior
    ///
    /// For `ExpirationDate::Days`:
    /// - If `use_fixed_time` is `true`: uses today at 18:30 UTC + days.
    /// - If `use_fixed_time` is `false`: uses reference datetime + days, or current time + days.
    ///
    /// For `ExpirationDate::DateTime`: returns the stored datetime directly.
    ///
    /// # Errors
    ///
    /// Returns `ExpirationDateError` if the time conversion fails (e.g., invalid
    /// hour/minute combination for the fixed time calculation).
    #[must_use = "discarding computed datetime defeats the purpose of the conversion"]
    pub fn get_date_with_options(
        &self,
        use_fixed_time: bool,
    ) -> Result<DateTime<Utc>, ExpirationDateError> {
        match self {
            ExpirationDate::Days(days) => {
                if use_fixed_time {
                    // Get today's date at 18:30 UTC (original behavior)
                    let today = Utc::now().date_naive();
                    let fixed_time = today.and_hms_opt(18, 30, 0).ok_or_else(|| {
                        ExpirationDateError::InvalidDateTime("invalid time".to_string())
                    })?;
                    let fixed_datetime =
                        DateTime::<Utc>::from_naive_utc_and_offset(fixed_time, Utc);
                    Ok(fixed_datetime + Duration::days((*days).to_i64()))
                } else {
                    // Check if we have a reference datetime stored
                    if let Some(ref_dt) = Self::get_reference_datetime() {
                        // Use the reference datetime and add the days
                        Ok(ref_dt + Duration::days((*days).to_i64()))
                    } else {
                        // Fallback to current time if no reference is stored
                        let now = Utc::now();
                        Ok(now + Duration::days((*days).to_i64()))
                    }
                }
            }
            ExpirationDate::DateTime(datetime) => Ok(*datetime),
        }
    }

    /// Returns the expiration date as a formatted string in `YYYY-MM-DD` format.
    ///
    /// # Errors
    ///
    /// Returns an error if the date calculation fails.
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
    #[must_use = "discarding the formatted date string makes this call a no-op"]
    pub fn get_date_string(&self) -> Result<String, ExpirationDateError> {
        // Use fixed time for backward compatibility with existing tests
        let date = self.get_date_with_options(true)?;
        Ok(date.format("%Y-%m-%d").to_string())
    }

    /// Creates an `ExpirationDate` from a string.
    ///
    /// Supports various formats:
    /// 1. **Positive number of days:** e.g., `"30.0"`
    /// 2. **ISO date:** e.g., `"2025-01-01"`
    /// 3. **Date with time and timezone:** e.g., `"2025-01-01 12:00:00 UTC"`
    /// 4. **RFC3339-like with time:** e.g., `"2025-05-23T15:29"`
    /// 5. **Numeric date (YYYYMMDD):** e.g., `"20250101"`
    /// 6. **Common date formats:** e.g., `"01-01-2025"`, `"30 jan 2025"`
    ///
    /// # Arguments
    ///
    /// * `s` - The input string to parse.
    ///
    /// # Errors
    ///
    /// Returns an error if no supported format can parse the string.
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
    /// ```
    #[must_use = "parsing has no side effects; use the parsed expiration date"]
    pub fn from_string(s: &str) -> Result<Self, ExpirationDateError> {
        // First try parsing as Positive (days)
        if let Ok(days) = s.parse::<Positive>() {
            return Ok(ExpirationDate::Days(days));
        }

        // Try to parse as a date only
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            let datetime = date
                .and_hms_opt(18, 30, 0)
                .ok_or_else(|| ExpirationDateError::InvalidDateTime("invalid time".to_string()))?;
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
            // Store the datetime as reference
            Self::set_reference_datetime(Some(utc_dt));
            return Ok(ExpirationDate::DateTime(utc_dt));
        }

        // Try to parse as a date with time and timezone
        if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %Z") {
            let utc_dt = dt.with_timezone(&Utc);
            // Store the datetime as reference
            Self::set_reference_datetime(Some(utc_dt));
            return Ok(ExpirationDate::DateTime(utc_dt));
        }

        // Try parsing format "2025-05-23 12:03:18 UTC"
        if s.contains(" UTC") && s.contains(':') {
            // Try various formats for the pattern with UTC
            for format in ["%Y-%m-%d %H:%M:%S %Z", "%Y-%m-%d %H:%M:%S UTC"] {
                if let Ok(datetime) = DateTime::parse_from_str(s, format) {
                    let utc_dt = DateTime::from(datetime);
                    // Store the datetime as reference
                    Self::set_reference_datetime(Some(utc_dt));
                    return Ok(ExpirationDate::DateTime(utc_dt));
                }
            }

            // If previous formats fail, try to build it manually
            if s.contains(" UTC") {
                // Extract the date and time part without the UTC suffix
                let date_time_part = s.trim_end_matches(" UTC").trim();

                // Try to parse as a date with time
                if let Ok(dt) = NaiveDateTime::parse_from_str(date_time_part, "%Y-%m-%d %H:%M:%S") {
                    let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
                    // Store the datetime as reference
                    Self::set_reference_datetime(Some(utc_dt));
                    return Ok(ExpirationDate::DateTime(utc_dt));
                }
            }
        }

        // Try parsing format "2025-05-23T15:29" (without seconds)
        if s.contains('T') && s.matches(':').count() == 1 {
            // Add seconds and time zone if not present
            let datetime_str = format!("{s}:00Z");
            if let Ok(datetime) = DateTime::parse_from_rfc3339(&datetime_str) {
                let utc_dt = DateTime::from(datetime);
                // Store the datetime as reference
                Self::set_reference_datetime(Some(utc_dt));
                return Ok(ExpirationDate::DateTime(utc_dt));
            }
        }

        // Try numeric date formats first
        if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
            // Format: YYYYMMDD
            let year = s[0..4].parse::<i32>()?;
            let month = s[4..6].parse::<u32>()?;
            let day = s[6..8].parse::<u32>()?;

            if let Some(naive_datetime) = NaiveDate::from_ymd_opt(year, month, day)
                .and_then(|date| date.and_hms_opt(23, 59, 59))
            {
                let datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc);
                return Ok(ExpirationDate::DateTime(datetime));
            }
        }

        // Try parsing common date formats, including ISO format
        let formats = [
            "%Y-%m-%d", // "2024-01-01"
            "%d-%m-%Y", // "01-01-2025"
            "%d %b %Y", // "30 jan 2025"
            "%d-%b-%Y", // "30-jan-2025"
            "%d %B %Y", // "30 january 2025"
            "%d-%B-%Y", // "30-january-2025"
        ];

        for format in formats {
            if let Ok(naive_date) = NaiveDate::parse_from_str(s.to_lowercase().as_str(), format) {
                // Convert NaiveDate to DateTime<Utc> by setting time to 18:30
                let naive_datetime = naive_date.and_hms_opt(18, 30, 0).ok_or_else(|| {
                    ExpirationDateError::InvalidDateTime(format!(
                        "invalid time conversion for date: {s}"
                    ))
                })?;

                let datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc);
                return Ok(ExpirationDate::DateTime(datetime));
            }
        }

        // If none of the above worked, return error
        Err(ExpirationDateError::ParseError(format!(
            "failed to parse expirationdate from string: {s}"
        )))
    }

    /// Converts a string representation of an expiration date into a `Days` variant.
    ///
    /// Parses the string using `from_string`, then converts the result to its
    /// equivalent in days.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice representing the expiration date.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing or conversion fails.
    #[must_use = "parsing has no side effects; use the parsed expiration days value"]
    pub fn from_string_to_days(s: &str) -> Result<Self, ExpirationDateError> {
        // Try to parse as a date
        let date_result = Self::from_string(s);
        if let Ok(expiration_date) = date_result {
            // Convert to days
            let days = expiration_date.get_days()?;
            // The get_days method will have stored the reference datetime if it was a DateTime variant
            return Ok(ExpirationDate::Days(days));
        }

        // If parsing as a date fails, try parsing as a number of days directly
        if let Ok(days) = s.parse::<Positive>() {
            // Clear any stored reference datetime since we're creating a Days variant directly
            Self::set_reference_datetime(None);
            return Ok(ExpirationDate::Days(days));
        }

        // If all parsing attempts fail, return an error
        Err(ExpirationDateError::ParseError(
            "failed to parse expiration date".to_string(),
        ))
    }
}

impl Default for ExpirationDate {
    fn default() -> Self {
        ExpirationDate::Days(positive::constants::DAYS_IN_A_YEAR)
    }
}

impl fmt::Display for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpirationDate::Days(days) => {
                // Use the stored reference datetime if available, otherwise use get_date_with_options
                if let Some(ref_dt) = ExpirationDate::get_reference_datetime() {
                    // Calculate the expiration date using the reference datetime
                    let expiration_date = ref_dt + Duration::days((*days).to_i64());
                    write!(f, "{}", expiration_date.format("%Y-%m-%d %H:%M:%S UTC"))
                } else if let Ok(date) = self.get_date_with_options(false) {
                    // Use the date from get_date_with_options with current time
                    write!(f, "{}", date.format("%Y-%m-%d %H:%M:%S UTC"))
                } else {
                    // Fallback if get_date_with_options fails
                    let duration = Duration::days((*days).to_i64());
                    let expiration = Utc::now() + duration;
                    write!(f, "{}", expiration.format("%Y-%m-%d %H:%M:%S UTC"))
                }
            }
            ExpirationDate::DateTime(date_time) => {
                write!(f, "{}", date_time.format("%Y-%m-%d %H:%M:%S UTC"))
            }
        }
    }
}

impl fmt::Debug for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpirationDate::Days(days) => write!(f, "ExpirationDate::Days({days:.2})"),
            ExpirationDate::DateTime(date_time) => {
                write!(f, "ExpirationDate::DateTime({date_time})")
            }
        }
    }
}

impl Serialize for ExpirationDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ExpirationDate::Days(days) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("days", &days.to_f64())?;
                state.end()
            }
            ExpirationDate::DateTime(dt) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("datetime", &dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ExpirationDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            days,
            datetime,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl Visitor<'_> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`days` or `datetime`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "days" => Ok(Field::days),
                            "datetime" => Ok(Field::datetime),
                            _ => Err(serde::de::Error::unknown_field(
                                value,
                                &["days", "datetime"],
                            )),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ExpirationDateVisitor;

        impl<'de> Visitor<'de> for ExpirationDateVisitor {
            type Value = ExpirationDate;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ExpirationDate")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ExpirationDate, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut days: Option<Positive> = None;
                let mut datetime: Option<DateTime<Utc>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::days => {
                            if days.is_some() {
                                return Err(serde::de::Error::duplicate_field("days"));
                            }
                            let value: f64 = map.next_value()?;
                            days = Some(Positive::new(value).map_err(serde::de::Error::custom)?);
                        }
                        Field::datetime => {
                            if datetime.is_some() {
                                return Err(serde::de::Error::duplicate_field("datetime"));
                            }
                            let value: String = map.next_value()?;
                            datetime = Some(
                                DateTime::parse_from_rfc3339(&value)
                                    .map_err(serde::de::Error::custom)?
                                    .with_timezone(&Utc),
                            );
                        }
                    }
                }

                if let Some(days) = days {
                    Ok(ExpirationDate::Days(days))
                } else if let Some(datetime) = datetime {
                    Ok(ExpirationDate::DateTime(datetime))
                } else {
                    Err(serde::de::Error::missing_field("either days or datetime"))
                }
            }
        }

        const FIELDS: &[&str] = &["days", "datetime"];
        deserializer.deserialize_struct("ExpirationDate", FIELDS, ExpirationDateVisitor)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_expiration_date {
    use super::*;
    use chrono::Duration;
    use positive::pos_or_panic;

    #[test]
    fn test_expiration_date_days() {
        let days_in_year = pos_or_panic!(365.0);
        let expiration = ExpirationDate::Days(days_in_year);
        assert_eq!(expiration.get_years().unwrap(), 1.0);

        let expiration = ExpirationDate::Days(pos_or_panic!(182.5));
        assert_eq!(expiration.get_years().unwrap(), 0.5);

        let expiration = ExpirationDate::Days(Positive::ZERO);
        assert_eq!(expiration.get_years().unwrap(), 0.0);
    }

    #[test]
    fn test_expiration_date_datetime() {
        // Test for a date exactly one year in the future
        let one_year_future = Utc::now() + Duration::days(365);
        let expiration = ExpirationDate::DateTime(one_year_future);
        assert!((expiration.get_years().unwrap().to_f64() - 1.0).abs() < 0.01);

        // Test for a date 6 months in the future
        let six_months_future = Utc::now() + Duration::days(182);
        let expiration = ExpirationDate::DateTime(six_months_future);
        assert!((expiration.get_years().unwrap().to_f64() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_expiration_date_datetime_specific() {
        let specific_date = Utc::now() + Duration::days(1);
        let expiration = ExpirationDate::DateTime(specific_date);
        assert!(expiration.get_years().unwrap() > Positive::ZERO);
    }

    #[test]
    fn test_get_date_from_datetime() {
        let future_date = Utc::now() + Duration::days(60);
        let expiration = ExpirationDate::DateTime(future_date);
        let result = expiration.get_date().unwrap();

        assert_eq!(result, future_date);
    }

    #[test]
    fn test_get_date_from_past_datetime() {
        let past_date = Utc::now() - Duration::days(30);
        let expiration = ExpirationDate::DateTime(past_date);
        let result = expiration.get_date().unwrap();
        assert_eq!(result, past_date);
    }

    #[test]
    fn test_positive_days() {
        let days_in_year = pos_or_panic!(365.0);
        let expiration = ExpirationDate::Days(days_in_year);
        let years = expiration.get_years().unwrap();
        assert_eq!(years, 1.0);
    }

    #[test]
    fn test_comparisons() {
        let one_day = ExpirationDate::Days(Positive::ONE);
        let less_than_one_day = ExpirationDate::Days(pos_or_panic!(0.99));

        assert!(less_than_one_day < one_day);

        let now = Utc::now();
        let future = now + Duration::days(1);
        let past = now - Duration::days(1);
        let future_date = ExpirationDate::DateTime(future);
        let past_date = ExpirationDate::DateTime(past);

        assert!(future_date > past_date);

        let ten_days = ExpirationDate::Days(Positive::TEN);
        let tomorrow = Utc::now() + Duration::days(1);
        let tomorrow_date = ExpirationDate::DateTime(tomorrow);
        assert!(tomorrow_date < ten_days);
    }

    #[test]
    fn test_default() {
        let default = ExpirationDate::default();
        match default {
            ExpirationDate::Days(days) => assert_eq!(days, pos_or_panic!(365.0)),
            _ => panic!("Expected Days variant"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_formatting {
    use super::*;
    use chrono::{Duration, TimeZone};
    use positive::pos_or_panic;

    #[test]
    fn test_get_date_string_days() {
        let today = Utc::now();
        let expiration = ExpirationDate::Days(pos_or_panic!(30.0));
        let date_str = expiration.get_date_string().unwrap();
        let expected_date = (today + Duration::days(30)).format("%Y-%m-%d").to_string();
        assert_eq!(date_str, expected_date);
    }

    #[test]
    fn test_get_date_string_datetime() {
        let specific_date = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
        let expiration = ExpirationDate::DateTime(specific_date);
        assert_eq!(expiration.get_date_string().unwrap(), "2024-12-31");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_from_string {
    use super::*;

    #[test]
    fn test_from_string_valid_days() {
        let result = ExpirationDate::from_string("30.0");
        assert!(result.is_ok());
        match result.unwrap() {
            ExpirationDate::Days(days) => assert_eq!(days, 30.0),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_from_string_passed_datetime() {
        let result = ExpirationDate::from_string("2024-12-31T00:00:00Z");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_string_format_one() {
        let result = ExpirationDate::from_string("30 jan 2025");
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_string_format_two() {
        let result = ExpirationDate::from_string("30-jan-2025");
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_string_format_three() {
        let result = ExpirationDate::from_string("20250101");
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_string_format_four() {
        let result = ExpirationDate::from_string("30-01-2025");
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_string_invalid_format() {
        let result = ExpirationDate::from_string("invalid date");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_string_with_time_no_seconds() {
        // Test format "2025-05-23T15:29"
        let date_str = "2025-05-23T15:29";
        let result = ExpirationDate::from_string(date_str).unwrap();
        if let ExpirationDate::DateTime(dt) = result {
            assert_eq!(dt.format("%Y-%m-%dT%H:%M").to_string(), "2025-05-23T15:29");
        } else {
            panic!("Expected DateTime variant");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_serialization {
    use super::*;
    use chrono::{TimeZone, Utc};
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
        let modified_serialized = serialized.replace("Days", "days");
        let deserialized: ExpirationDate = serde_json::from_str(&modified_serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_expiration_date_roundtrip_datetime() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        let original = ExpirationDate::DateTime(dt);
        let serialized = serde_json::to_string(&original).unwrap();
        let modified_serialized = serialized.replace("DateTime", "datetime");
        let deserialized: ExpirationDate = serde_json::from_str(&modified_serialized).unwrap();
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_hash {
    use super::*;
    use chrono::{Duration, TimeZone};

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

        // Even though they might represent the same expiration in practice,
        // they should hash differently because they're different variants
        assert_ne!(calculate_hash(&exp1), calculate_hash(&exp2));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests_comparisons {
    use super::*;

    use chrono::{Duration, TimeZone, Utc};
    use positive::pos_or_panic;
    use rust_decimal_macros::dec;
    use std::cmp::Ordering;

    #[test]
    fn test_partial_eq_days_variants_equal() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_days_variants_within_epsilon() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 =
            ExpirationDate::Days(Positive::new_decimal(dec!(30.0) + EPSILON / dec!(2.0)).unwrap());
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_days_variants_outside_epsilon() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.1));
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_partial_eq_datetime_variants_equal() {
        let datetime = Utc.with_ymd_and_hms(2024, 12, 15, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime);
        let date2 = ExpirationDate::DateTime(datetime);
        assert_eq!(date1, date2);
    }

    #[test]
    fn test_partial_eq_datetime_variants_different() {
        let datetime1 = Utc.with_ymd_and_hms(2027, 12, 15, 16, 0, 0).unwrap();
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 16, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime1);
        let date2 = ExpirationDate::DateTime(datetime2);
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_partial_eq_mixed_variants_with_zero_fallback() {
        let days_date = ExpirationDate::Days(Positive::ZERO);

        // Create a past DateTime that should result in ZERO days
        let past_datetime = Utc::now() - chrono::Duration::days(10);
        let datetime_date = ExpirationDate::DateTime(past_datetime);

        // Both should be equal when they fall back to ZERO
        assert_eq!(days_date, datetime_date);
    }

    #[test]
    fn test_eq_trait_consistency() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date3 = ExpirationDate::Days(pos_or_panic!(30.0));

        // Reflexive: a == a
        assert_eq!(date1, date1);

        // Symmetric: if a == b, then b == a
        assert_eq!(date1, date2);
        assert_eq!(date2, date1);

        // Transitive: if a == b and b == c, then a == c
        assert_eq!(date1, date2);
        assert_eq!(date2, date3);
        assert_eq!(date1, date3);
    }

    #[test]
    fn test_partial_ord_returns_some() {
        let date1 = ExpirationDate::Days(pos_or_panic!(15.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));

        let result = date1.partial_cmp(&date2);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Ordering::Less);
    }

    #[test]
    fn test_ord_days_variants_less() {
        let date1 = ExpirationDate::Days(pos_or_panic!(15.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));

        assert_eq!(date1.cmp(&date2), Ordering::Less);
    }

    #[test]
    fn test_ord_days_variants_greater() {
        let date1 = ExpirationDate::Days(pos_or_panic!(45.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));

        assert_eq!(date1.cmp(&date2), Ordering::Greater);

        let date1 = ExpirationDate::Days(pos_or_panic!(45.0));
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);

        assert_eq!(date1.cmp(&date2), Ordering::Less);

        let datetime1 = Utc.with_ymd_and_hms(2026, 12, 20, 16, 0, 0).unwrap();
        let date1 = ExpirationDate::DateTime(datetime1);
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);

        assert_eq!(date1.cmp(&date2), Ordering::Less);

        let date1 = ExpirationDate::Days(pos_or_panic!(3000.0));
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 20, 16, 0, 0).unwrap();
        let date2 = ExpirationDate::DateTime(datetime2);

        assert_eq!(date1.cmp(&date2), Ordering::Greater);
    }

    #[test]
    fn test_ord_days_variants_equal() {
        let date1 = ExpirationDate::Days(pos_or_panic!(30.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(30.0));

        assert_eq!(date1.cmp(&date2), Ordering::Equal);
    }

    #[test]
    fn test_ord_datetime_variants() {
        let datetime1 = Utc.with_ymd_and_hms(2027, 12, 15, 16, 0, 0).unwrap();
        let datetime2 = Utc.with_ymd_and_hms(2027, 12, 20, 16, 0, 0).unwrap();

        let date1 = ExpirationDate::DateTime(datetime1);
        let date2 = ExpirationDate::DateTime(datetime2);

        let result = date1.cmp(&date2);
        assert!(result != Ordering::Equal);
    }

    #[test]
    fn test_ord_mixed_variants() {
        let days_date = ExpirationDate::Days(pos_or_panic!(20.0));
        let future_datetime = Utc::now() + Duration::days(30);
        let datetime_date = ExpirationDate::DateTime(future_datetime);

        let result = days_date.cmp(&datetime_date);
        assert!(result != Ordering::Equal);
        assert_eq!(result, Ordering::Less);
    }

    #[test]
    fn test_ord_with_zero_fallback() {
        let date1 = ExpirationDate::Days(Positive::ZERO);
        let date2 = ExpirationDate::Days(pos_or_panic!(10.0));

        assert_eq!(date1.cmp(&date2), Ordering::Less);
        assert_eq!(date2.cmp(&date1), Ordering::Greater);
    }

    #[test]
    fn test_ord_consistency_with_partial_ord() {
        let date1 = ExpirationDate::Days(pos_or_panic!(25.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(35.0));

        let ord_result = date1.cmp(&date2);
        let partial_ord_result = date1.partial_cmp(&date2);

        assert_eq!(Some(ord_result), partial_ord_result);
    }

    #[test]
    fn test_ord_transitivity() {
        let date1 = ExpirationDate::Days(pos_or_panic!(10.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(20.0));
        let date3 = ExpirationDate::Days(pos_or_panic!(30.0));

        assert_eq!(date1.cmp(&date2), Ordering::Less);
        assert_eq!(date2.cmp(&date3), Ordering::Less);
        assert_eq!(date1.cmp(&date3), Ordering::Less);
    }

    #[test]
    fn test_ord_antisymmetry() {
        let date1 = ExpirationDate::Days(pos_or_panic!(25.0));
        let date2 = ExpirationDate::Days(pos_or_panic!(25.0));

        assert!(date1.cmp(&date2) <= Ordering::Equal);
        assert!(date2.cmp(&date1) <= Ordering::Equal);
        assert_eq!(date1.cmp(&date2), Ordering::Equal);
    }

    #[test]
    fn test_ord_reflexivity() {
        let date = ExpirationDate::Days(pos_or_panic!(25.0));
        assert_eq!(date.cmp(&date), Ordering::Equal);

        let datetime = Utc.with_ymd_and_hms(2024, 12, 15, 16, 0, 0).unwrap();
        let datetime_date = ExpirationDate::DateTime(datetime);
        assert_eq!(datetime_date.cmp(&datetime_date), Ordering::Equal);
    }

    #[test]
    fn test_sorting_expiration_dates() {
        let mut dates = vec![
            ExpirationDate::Days(pos_or_panic!(45.0)),
            ExpirationDate::Days(pos_or_panic!(15.0)),
            ExpirationDate::Days(pos_or_panic!(30.0)),
            ExpirationDate::Days(pos_or_panic!(5.0)),
        ];

        dates.sort();

        let expected = vec![
            ExpirationDate::Days(pos_or_panic!(5.0)),
            ExpirationDate::Days(pos_or_panic!(15.0)),
            ExpirationDate::Days(pos_or_panic!(30.0)),
            ExpirationDate::Days(pos_or_panic!(45.0)),
        ];

        assert_eq!(dates, expected);
    }

    #[test]
    fn test_partial_eq_edge_case_epsilon_boundary() {
        let base_value = Positive::HUNDRED;
        let date1 = ExpirationDate::Days(base_value);
        let date2 =
            ExpirationDate::Days(Positive::new_decimal(base_value.value() + EPSILON).unwrap());

        // Should not be equal as difference equals epsilon (not less than)
        assert_ne!(date1, date2);
    }

    #[test]
    fn test_mixed_variant_comparison_edge_cases() {
        let zero_days = ExpirationDate::Days(Positive::ZERO);

        let very_old_datetime = Utc.with_ymd_and_hms(1990, 1, 1, 0, 0, 0).unwrap();
        let old_datetime_date = ExpirationDate::DateTime(very_old_datetime);

        // Both should fall back to ZERO and be equal
        assert_eq!(zero_days, old_datetime_date);
    }
}
