//! Conversion and formatting helpers for [`ExpirationDate`].
//!
//! Covers fractional-day / year accessors, absolute-datetime resolution
//! (including fixed-time and caller-supplied bases), and the `Display` /
//! `Default` impls. These live here so the hot path (`get_days`, `get_years`)
//! is easy to locate and audit.

use crate::ExpirationDate;
use crate::conventions::{Actual365Fixed, DayCount};
use crate::error::ExpirationDateError;
use chrono::{DateTime, Duration, Utc};
use positive::Positive;
use positive::constants::DAYS_IN_A_YEAR;
use std::fmt;

impl ExpirationDate {
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

    pub(crate) fn get_date_with_base(
        &self,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, ExpirationDateError> {
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
