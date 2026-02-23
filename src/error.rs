use thiserror::Error;

/// Error types for expiration date operations.
#[derive(Debug, Error)]
pub enum ExpirationDateError {
    /// Failed to parse a string into an ExpirationDate.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Failure during numeric or date conversion.
    #[error("Conversion error from {from_type} to {to_type}: {reason}")]
    ConversionError {
        /// The source type of the conversion.
        from_type: String,
        /// The target type of the conversion.
        to_type: String,
        /// The detailed reason for the failure.
        reason: String,
    },

    /// Provided datetime is invalid for the context.
    #[error("Invalid datetime: {0}")]
    InvalidDateTime(String),

    /// Error from the underlying Positive type.
    #[error("Positive error: {0}")]
    PositiveError(#[from] positive::error::PositiveError),

    /// Error parsing dates using the chrono crate.
    #[error("Chrono parse error: {0}")]
    ChronoParseError(#[from] chrono::ParseError),

    /// Error parsing integers from strings.
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    
    /// Numeric overflow during financial convention calculations.
    #[error("Arithmetic overflow in convention calculation")]
    ArithmeticOverflow,
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
