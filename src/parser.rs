//! String parsers for [`ExpirationDate`].
//!
//! Houses [`ExpirationDate::from_string`] and [`ExpirationDate::from_string_to_days`],
//! which accept a range of calendar and numeric formats. See the method docs
//! for the exact grammar — that documentation is part of the public contract.

use crate::ExpirationDate;
use crate::error::ExpirationDateError;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use positive::Positive;

impl ExpirationDate {
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
