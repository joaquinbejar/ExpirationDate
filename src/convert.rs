//! Conversion and formatting helpers for [`ExpirationDate`].
//!
//! Covers fractional-day / year accessors, absolute-datetime resolution
//! (including fixed-time and caller-supplied bases), and the `Display` /
//! `Default` impls. These live here so the hot path (`get_days`, `get_years`)
//! is easy to locate and audit.
//!
//! # Reading the clock
//!
//! Every conversion comes in two forms:
//!
//! - the `*_at` methods ([`ExpirationDate::get_date_at`],
//!   [`ExpirationDate::get_days_at`], [`ExpirationDate::get_years_at`],
//!   [`ExpirationDate::get_years_with_convention_at`],
//!   [`ExpirationDate::get_date_string_at`]) take the time as an argument.
//!   They are pure, ignore the thread's reference datetime, and exist in
//!   every build;
//! - the methods without a suffix ask the system for the time. They exist
//!   only with the `clock` feature, so a build without it cannot call them.
//!
//! `Display` exists in every build. For a `Days` it reads the base as the
//! documentation of that impl describes.

use crate::ExpirationDate;
use crate::conventions::{Actual365Fixed, DayCount};
use crate::error::ExpirationDateError;
use chrono::{DateTime, TimeDelta, Utc};
use positive::Positive;
use positive::constants::DAYS_IN_A_YEAR;
use std::fmt;

/// Seconds in one day, for turning a span into fractional days.
const SECONDS_PER_DAY: f64 = 86_400.0;

/// The instant `days` whole days after `base`; any fraction of a day is
/// dropped.
fn offset_whole_days(
    base: DateTime<Utc>,
    days: Positive,
) -> Result<DateTime<Utc>, ExpirationDateError> {
    let overflow = || {
        ExpirationDateError::ArithmeticOverflow(format!(
            "{days} days after {base} cannot be represented"
        ))
    };
    let span = TimeDelta::try_days(i64::try_from(days)?).ok_or_else(overflow)?;
    base.checked_add_signed(span).ok_or_else(overflow)
}

/// The fractional days from `now` to `target`, zero once `target` has passed.
fn days_until(target: DateTime<Utc>, now: DateTime<Utc>) -> Result<Positive, ExpirationDateError> {
    let num_days = target.signed_duration_since(now).num_seconds() as f64 / SECONDS_PER_DAY;
    if num_days <= 0.0 {
        return Ok(Positive::ZERO);
    }
    Positive::new(num_days).map_err(Into::into)
}

/// The year fraction from `now` to `target`, zero once `target` has passed.
fn years_until<C: DayCount>(
    convention: C,
    target: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Positive, ExpirationDateError> {
    if target <= now {
        return Ok(Positive::ZERO);
    }
    let fraction = convention.year_fraction(&now, &target)?;
    Positive::new(fraction).map_err(Into::into)
}

/// A day count as years, on the 365-day year of `get_years`.
fn days_to_years(days: Positive) -> Result<Positive, ExpirationDateError> {
    Positive::new(days.to_f64() / DAYS_IN_A_YEAR.to_f64()).map_err(Into::into)
}

/// 18:30 UTC on the calendar day of `today`, the base of the fixed-time forms.
fn fixed_time_base(today: DateTime<Utc>) -> Result<DateTime<Utc>, ExpirationDateError> {
    let fixed = today
        .date_naive()
        .and_hms_opt(18, 30, 0)
        .ok_or_else(|| ExpirationDateError::InvalidDateTime("Fixed time error".into()))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(fixed, Utc))
}

impl ExpirationDate {
    /// Resolves the expiration to an instant, counting a `Days` from `base`.
    ///
    /// A `DateTime` is returned as it is. A `Days` adds its whole days to
    /// `base`, dropping any fraction of a day, as [`Self::get_date`] does.
    /// Pure: reads neither the clock nor the thread's reference datetime.
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ArithmeticOverflow] if the instant cannot
    /// be represented, or [ExpirationDateError::PositiveError] if the day
    /// count does not fit an `i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{TimeZone, Utc};
    /// use expiration_date::ExpirationDate;
    /// use positive::pos_or_panic;
    ///
    /// let base = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    /// let resolved = ExpirationDate::Days(pos_or_panic!(30.0))
    ///     .get_date_at(base)
    ///     .unwrap();
    /// assert_eq!(resolved, Utc.with_ymd_and_hms(2030, 1, 31, 0, 0, 0).unwrap());
    /// ```
    #[must_use = "Resolved date must be used for settlement or further calculations."]
    pub fn get_date_at(&self, base: DateTime<Utc>) -> Result<DateTime<Utc>, ExpirationDateError> {
        match self {
            Self::Days(days) => offset_whole_days(base, *days),
            Self::DateTime(dt) => Ok(*dt),
        }
    }

    /// Returns the fractional days left at the instant `now`.
    ///
    /// A `Days` returns its own count. A `DateTime` returns the days from
    /// `now` to it, zero once it has passed. Pure: reads neither the clock nor
    /// the thread's reference datetime.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if conversion to Positive fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Duration, TimeZone, Utc};
    /// use expiration_date::ExpirationDate;
    /// use positive::pos_or_panic;
    ///
    /// let now = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    /// let expiry = ExpirationDate::DateTime(now + Duration::days(30));
    /// assert_eq!(expiry.get_days_at(now).unwrap(), pos_or_panic!(30.0));
    /// ```
    #[must_use = "Calculated days must be used to ensure valid financial models."]
    pub fn get_days_at(&self, now: DateTime<Utc>) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => Ok(*days),
            Self::DateTime(dt) => days_until(*dt, now),
        }
    }

    /// Returns the years left at the instant `now`, on Actual/365 Fixed.
    ///
    /// Pure: reads neither the clock nor the thread's reference datetime.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    pub fn get_years_at(&self, now: DateTime<Utc>) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => days_to_years(*days),
            Self::DateTime(_) => self.get_years_with_convention_at(Actual365Fixed, now),
        }
    }

    /// Returns the years left at the instant `now`, on a day count convention.
    ///
    /// A `Days` counts from `now`. Pure: reads neither the clock nor the
    /// thread's reference datetime.
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    pub fn get_years_with_convention_at<C: DayCount>(
        &self,
        convention: C,
        now: DateTime<Utc>,
    ) -> Result<Positive, ExpirationDateError> {
        years_until(convention, self.get_date_at(now)?, now)
    }

    /// Returns the expiration as a YYYY-MM-DD string, given the current day.
    ///
    /// A `Days` counts from 18:30 UTC on the calendar day of `today`, as
    /// [`Self::get_date_string`] does. Pure: reads neither the clock nor the
    /// thread's reference datetime.
    ///
    /// # Errors
    /// Returns error if the date cannot be resolved.
    #[must_use = "Formatted string should be consumed for display or reporting."]
    pub fn get_date_string_at(&self, today: DateTime<Utc>) -> Result<String, ExpirationDateError> {
        let date = match self {
            Self::Days(days) => offset_whole_days(fixed_time_base(today)?, *days)?,
            Self::DateTime(dt) => *dt,
        };
        Ok(date.format("%Y-%m-%d").to_string())
    }
}

/// Conversions that take the time from the system.
///
/// All of them need the `clock` feature; each has a `*_at` form that takes the
/// time as an argument and is pure.
#[cfg(feature = "clock")]
#[cfg_attr(docsrs, doc(cfg(feature = "clock")))]
impl ExpirationDate {
    /// Calculates years using a specific day count convention.
    ///
    /// Reads the clock for both variants: it is where the year fraction
    /// starts. A `Days` counts from the thread's reference datetime when one
    /// is set. For a pure form see [`Self::get_years_with_convention_at`].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if calculation fails.
    #[must_use = "Calculated years must be used to ensure valid financial models."]
    pub fn get_years_with_convention<C: DayCount>(
        &self,
        convention: C,
    ) -> Result<Positive, ExpirationDateError> {
        let now = Utc::now();
        let target = match self {
            Self::Days(days) => {
                offset_whole_days(Self::get_reference_datetime().unwrap_or(now), *days)?
            }
            Self::DateTime(dt) => *dt,
        };
        years_until(convention, target, now)
    }

    /// Calculates years until expiration using standard Actual/365 Fixed.
    ///
    /// Reads the clock for a `DateTime`; a `Days` needs no base. For a pure
    /// form see [`Self::get_years_at`].
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
            Self::Days(days) => days_to_years(*days),
            Self::DateTime(_) => self.get_years_with_convention(Actual365Fixed),
        }
    }

    /// Returns the number of fractional days until expiration.
    ///
    /// Reads the clock for a `DateTime`; a `Days` returns its own count. For a
    /// pure form see [`Self::get_days_at`].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if conversion to Positive fails.
    #[must_use = "Calculated days must be used to ensure valid financial models."]
    #[inline]
    pub fn get_days(&self) -> Result<Positive, ExpirationDateError> {
        match self {
            Self::Days(days) => Ok(*days),
            Self::DateTime(dt) => days_until(*dt, Utc::now()),
        }
    }

    /// Resolves expiration to an absolute [`DateTime<Utc>`].
    ///
    /// A `DateTime` is returned as it is, without reading the clock. A `Days`
    /// counts from the thread's reference datetime, or from the clock when
    /// none is set. For a pure form see [`Self::get_date_at`].
    ///
    /// # Errors
    /// Returns [ExpirationDateError::ArithmeticOverflow] if the instant cannot
    /// be represented.
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

    /// Resolves datetime with advanced base-time options.
    ///
    /// A `DateTime` is returned as it is, without reading the clock. With
    /// `use_fixed_time`, a `Days` counts from 18:30 UTC today by the clock,
    /// ignoring the reference datetime; otherwise as [`Self::get_date`]. For a
    /// pure form see [`Self::get_date_at`].
    ///
    /// # Errors
    /// Returns [ExpirationDateError] if the construction fails.
    #[must_use = "Resolved date must be used for settlement or further calculations."]
    pub fn get_date_with_options(
        &self,
        use_fixed_time: bool,
    ) -> Result<DateTime<Utc>, ExpirationDateError> {
        let days = match self {
            Self::Days(days) => *days,
            Self::DateTime(dt) => return Ok(*dt),
        };
        let base = if use_fixed_time {
            fixed_time_base(Utc::now())?
        } else {
            Self::get_reference_datetime().unwrap_or_else(Utc::now)
        };
        offset_whole_days(base, days)
    }

    /// Returns the expiration date as a formatted YYYY-MM-DD string.
    ///
    /// A `DateTime` is formatted without reading the clock; a `Days` counts
    /// from 18:30 UTC today by the clock. For a pure form see
    /// [`Self::get_date_string_at`].
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

/// The base `Display` counts a `Days` from: the thread's reference datetime,
/// or the clock when none is set.
#[cfg(feature = "clock")]
fn display_base() -> Option<DateTime<Utc>> {
    Some(ExpirationDate::get_reference_datetime().unwrap_or_else(Utc::now))
}

/// The base `Display` counts a `Days` from: the thread's reference datetime.
#[cfg(not(feature = "clock"))]
fn display_base() -> Option<DateTime<Utc>> {
    ExpirationDate::get_reference_datetime()
}

/// A `DateTime` prints its instant and never reads the clock.
///
/// A `Days` prints the instant it resolves to from the thread's reference
/// datetime. With no reference set it reads the clock when the `clock` feature
/// is on, and otherwise prints the count, as in `30 days`. Features unify
/// across a build, so another crate that turns `clock` on changes that output;
/// for text that cannot vary, format [`ExpirationDate::get_date_at`] instead.
impl fmt::Display for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resolved = match self {
            Self::DateTime(dt) => Ok(*dt),
            Self::Days(days) => match display_base() {
                Some(base) => offset_whole_days(base, *days),
                None => return write!(f, "{days} days"),
            },
        };
        match resolved {
            Ok(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S UTC")),
            Err(_) => write!(f, "Invalid Expiration"),
        }
    }
}
