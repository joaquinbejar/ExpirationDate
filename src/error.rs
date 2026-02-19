//! # Error Module
//!
//! Provides error types for expiration date operations including parsing,
//! conversion, and date calculation failures.

use thiserror::Error;

/// Represents errors that can occur during expiration date operations.
///
/// This enum covers parsing failures, conversion issues, and invalid
/// date/time values encountered when working with `ExpirationDate`.
#[derive(Error, Debug)]
pub enum ExpirationDateError {
    /// Error when parsing a string into an expiration date.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Error when converting between expiration date representations.
    #[error("conversion error from {from_type} to {to_type}: {reason}")]
    ConversionError {
        /// The source type being converted from.
        from_type: String,
        /// The destination type being converted to.
        to_type: String,
        /// Detailed explanation of why the conversion failed.
        reason: String,
    },

    /// Error when a date or time value is invalid.
    #[error("invalid date/time: {0}")]
    InvalidDateTime(String),

    /// Error originating from the positive crate.
    #[error(transparent)]
    PositiveError(#[from] positive::PositiveError),

    /// Error originating from chrono parsing.
    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),

    /// Error when parsing an integer fails.
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl From<String> for ExpirationDateError {
    fn from(msg: String) -> Self {
        ExpirationDateError::ParseError(msg)
    }
}

impl From<&str> for ExpirationDateError {
    fn from(msg: &str) -> Self {
        ExpirationDateError::ParseError(msg.to_string())
    }
}
