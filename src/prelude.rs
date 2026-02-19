//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used types and traits from the
//! `expiration_date` crate, allowing users to import everything they need
//! with a single `use` statement:
//!
//! ```rust
//! use expiration_date::prelude::*;
//! ```

pub use crate::EPSILON;
pub use crate::ExpirationDate;
pub use crate::error::ExpirationDateError;
