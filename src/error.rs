//! # Error Module
//!
//! Provides error types for expiration date operations including parsing,
//! conversion, and date calculation failures.

use thiserror::Error;

/// Error types for expiration date operations.
#[derive(Debug, Error)]
pub enum ExpirationDateError {
    /// Failed to parse a string into an ExpirationDate.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Failure during numeric or date conversion.
    #[error("conversion error from {from_type} to {to_type}: {reason}")]
    ConversionError {
        /// The source type of the conversion.
        from_type: String,
        /// The target type of the conversion.
        to_type: String,
        /// The detailed reason for the failure.
        reason: String,
    },

    /// Provided datetime is invalid for the context.
    #[error("invalid datetime: {0}")]
    InvalidDateTime(String),

    /// Error from the underlying Positive type.
    #[error("positive error: {0}")]
    PositiveError(#[from] positive::error::PositiveError),

    /// Error parsing dates using the chrono crate.
    #[error("chrono parse error: {0}")]
    ChronoParseError(#[from] chrono::ParseError),

    /// Error parsing integers from strings.
    #[error("parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// Numeric overflow during financial convention calculations.
    #[error("arithmetic overflow: {0}")]
    ArithmeticOverflow(String),
}

impl From<String> for ExpirationDateError {
    fn from(s: String) -> Self {
        Self::ParseError(s)
    }
}

impl From<&str> for ExpirationDateError {
    fn from(s: &str) -> Self {
        Self::ParseError(s.to_string())
    }
}
