//! Hand-written comparison and hashing impls for [`ExpirationDate`].
//!
//! Equality, hashing and ordering are pure: they read neither the clock nor
//! the thread's reference datetime, so a sorted collection or a hash map stays
//! valid as time passes.
//!
//! - two `DateTime`s are equal when they are the same instant, to the
//!   nanosecond, past or future;
//! - two `Days` are equal when their day counts are equal (exactly, so that
//!   equality is transitive and agrees with `Hash`);
//! - a `Days` is never equal to a `DateTime`. Whether `Days(30)` and
//!   "30 days from now" name the same instant depends on when the question
//!   is asked, and an equality that changes with the clock cannot agree with
//!   a hash.
//!
//! `Hash` hashes the variant and its value, so `a == b` implies
//! `hash(a) == hash(b)`.
//!
//! `Ord` is total. Within a variant it is chronological: `Days` by day count,
//! `DateTime` by instant. Across variants every `Days` comes before every
//! `DateTime`, which says nothing about which one expires first: that depends
//! on a base. To compare two expirations in time whatever their variants, use
//! [`ExpirationDate::cmp_at`] with an explicit one.
//!
//! These impls are hand-written on purpose; do not replace them with
//! `#[derive]`, which would order the variants before their instants.

use crate::ExpirationDate;
use chrono::{DateTime, TimeDelta, Utc};
use positive::Positive;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Seconds in one day, for resolving a `Days` count to an instant.
const SECONDS_PER_DAY: Decimal = Decimal::from_parts(86_400, 0, 0, false, 0);

/// Nanoseconds in one second.
const NANOS_PER_SECOND: Decimal = Decimal::from_parts(1_000_000_000, 0, 0, false, 0);

impl ExpirationDate {
    /// Hash and tie-break tag of the variant.
    #[inline]
    const fn variant_tag(&self) -> u8 {
        match self {
            Self::Days(_) => 0,
            Self::DateTime(_) => 1,
        }
    }

    /// Compares two expirations in time, counting a `Days` from `base`.
    ///
    /// Same variants compare as [`Ord`] does. A `Days` against a `DateTime`
    /// compares the instants, to the nanosecond; when they coincide the `Days`
    /// comes first, so the result is `Equal` only for values that are `==`. A
    /// `Days` too large to resolve comes after every `DateTime`. Pure: reads
    /// neither the clock nor the thread's reference datetime.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Duration, TimeZone, Utc};
    /// use expiration_date::ExpirationDate;
    /// use positive::pos_or_panic;
    /// use std::cmp::Ordering;
    ///
    /// let base = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    /// let days = ExpirationDate::Days(pos_or_panic!(30.0));
    /// let earlier = ExpirationDate::DateTime(base + Duration::days(10));
    ///
    /// assert_eq!(days.cmp_at(&earlier, base), Ordering::Greater);
    /// // `Ord` needs no base: every `Days` comes before every `DateTime`.
    /// assert_eq!(days.cmp(&earlier), Ordering::Less);
    /// ```
    #[must_use = "The comparison result must be used."]
    pub fn cmp_at(&self, other: &Self, base: DateTime<Utc>) -> Ordering {
        match (self, other) {
            (Self::Days(days), Self::DateTime(instant)) => {
                cmp_days_to_instant(base, *days, instant)
            }
            (Self::DateTime(instant), Self::Days(days)) => {
                cmp_days_to_instant(base, *days, instant).reverse()
            }
            _ => self.cmp(other),
        }
    }
}

/// The instant `days` after `base`, to the nanosecond.
///
/// `None` when the instant lies past the last one `DateTime<Utc>` can hold;
/// such a `Days` comes after every `DateTime`.
fn resolve_days(base: DateTime<Utc>, days: Positive) -> Option<DateTime<Utc>> {
    let seconds = days.to_dec().checked_mul(SECONDS_PER_DAY)?;
    let whole = seconds.trunc();
    let nanos = seconds
        .checked_sub(whole)?
        .checked_mul(NANOS_PER_SECOND)?
        .round()
        .to_u32()?;
    let offset = TimeDelta::try_seconds(whole.to_i64()?)?
        .checked_add(&TimeDelta::nanoseconds(i64::from(nanos)))?;
    base.checked_add_signed(offset)
}

/// Orders `Days(days)`, counted from `base`, against `DateTime(instant)`.
///
/// A tie goes to the `Days`, so the result is never `Equal`.
fn cmp_days_to_instant(base: DateTime<Utc>, days: Positive, instant: &DateTime<Utc>) -> Ordering {
    match resolve_days(base, days) {
        Some(resolved) => resolved.cmp(instant).then(Ordering::Less),
        None => Ordering::Greater,
    }
}

impl Hash for ExpirationDate {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.variant_tag());
        match self {
            Self::Days(d) => d.hash(state),
            Self::DateTime(dt) => {
                dt.timestamp().hash(state);
                dt.timestamp_subsec_nanos().hash(state);
            }
        }
    }
}

impl PartialEq for ExpirationDate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Days(s), Self::Days(o)) => s == o,
            (Self::DateTime(s), Self::DateTime(o)) => s == o,
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
        match (self, other) {
            (Self::Days(s), Self::Days(o)) => s.cmp(o),
            (Self::DateTime(s), Self::DateTime(o)) => s.cmp(o),
            (Self::Days(_), Self::DateTime(_)) => Ordering::Less,
            (Self::DateTime(_), Self::Days(_)) => Ordering::Greater,
        }
    }
}
