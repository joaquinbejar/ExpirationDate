//! # ExpirationDate
//!
//! A professional high-performance financial instrument expiration date management library.
//!
//! ## Module layout
//!
//! The public API lives on [`ExpirationDate`]; the implementation is split across
//! focused submodules: [`cmp`] (hand-written `Hash` / `Eq` / `Ord`),
//! [`convert`] (day/year accessors, `Display`, `Default`), [`parser`]
//! (`from_string` and friends), and [`serde_impl`] (hand-written serde).

/// Hand-written comparison and hashing impls for [`ExpirationDate`].
pub mod cmp;
/// Financial day count conventions module.
pub mod conventions;
/// Conversion and formatting helpers for [`ExpirationDate`].
pub mod convert;
/// Error handling module for expiration date operations.
pub mod error;
/// String parsers for [`ExpirationDate`].
pub mod parser;
/// Prelude module for common traits and types.
pub mod prelude;
/// Hand-written serde impls for [`ExpirationDate`].
pub mod serde_impl;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use positive::Positive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

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
}
