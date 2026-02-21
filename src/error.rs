//! Unified error handling for ExpirationDate.

use std::fmt;
use std::error::Error;

/// Error types for expiration date operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpirationDateError {
    /// Failed to parse a string into an ExpirationDate.
    ParseError(String),
    /// Failure during numeric or date conversion.
    ConversionError { 
        from_type: String, 
        to_type: String, 
        reason: String 
    },
    /// Provided datetime is invalid for the context.
    InvalidDateTime(String),
    /// Error parsing integers from string formats.
    ParseIntError,
}

impl fmt::Display for ExpirationDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(s) => write!(f, "Parse error: {}", s),
            Self::ConversionError { from_type, to_type, reason } => {
                write!(f, "Conversion error from {} to {}: {}", from_type, to_type, reason)
            },
            Self::InvalidDateTime(s) => write!(f, "Invalid datetime: {}", s),
            Self::ParseIntError => write!(f, "Failed to parse integer component"),
        }
    }
}

impl Error for ExpirationDateError {}

impl From<std::num::ParseIntError> for ExpirationDateError {
    #[inline]
    #[cold]
    fn from(_: std::num::ParseIntError) -> Self {
        Self::ParseIntError
    }
}

impl From<positive::error::PositiveError> for ExpirationDateError {
    #[inline]
    #[cold]
    fn from(e: positive::error::PositiveError) -> Self {
        Self::ConversionError {
            from_type: "f64".into(),
            to_type: "Positive".into(),
            reason: e.to_string(),
        }
    }
}
