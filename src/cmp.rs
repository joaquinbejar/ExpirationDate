//! Hand-written comparison and hashing impls for [`ExpirationDate`].
//!
//! Every expiration stands for an instant: a `DateTime` is that instant, and
//! a `Days(d)` is `d` days after its base, which is the thread's reference
//! datetime ([`ExpirationDate::set_reference_datetime`]) or, when none is
//! set, `Utc::now()`. Ordering follows that resolved instant.
//!
//! Equality and hashing never read the clock:
//!
//! - two `DateTime`s are equal when they are the same instant, to the
//!   nanosecond, past or future;
//! - two `Days` are equal when their day counts are equal (exactly, so that
//!   equality is transitive and agrees with `Hash`); both resolve against the
//!   same base, so this is the same as comparing their instants;
//! - a `Days` is never equal to a `DateTime`. Whether `Days(30)` and
//!   "30 days from now" name the same instant depends on when the question
//!   is asked, and an equality that changes with the clock cannot agree with
//!   a hash.
//!
//! `Hash` hashes the variant and its value, so `a == b` implies
//! `hash(a) == hash(b)`.
//!
//! `Ord` is total. Within a variant it is clock-free: `Days` by day count,
//! `DateTime` by instant. Across variants it compares the resolved instants
//! and, when they coincide, puts the `Days` first, so it never answers
//! `Equal` for two values that are not `==`. A mixed comparison reads the
//! base, so its answer can change as time passes; an ordered collection that
//! holds both variants should pin the base with
//! [`ExpirationDate::set_reference_datetime`] or hold one variant only.
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

/// Orders `Days(days)` against `DateTime(instant)` by resolved instant.
///
/// The `Days` resolves against the thread's reference datetime, or
/// `Utc::now()` when none is set. A tie goes to the `Days`, so the result is
/// never `Equal`.
fn cmp_days_to_instant(days: Positive, instant: &DateTime<Utc>) -> Ordering {
    let base = ExpirationDate::get_reference_datetime().unwrap_or_else(Utc::now);
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
            (Self::Days(days), Self::DateTime(instant)) => cmp_days_to_instant(*days, instant),
            (Self::DateTime(instant), Self::Days(days)) => {
                cmp_days_to_instant(*days, instant).reverse()
            }
        }
    }
}
