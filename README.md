[![Dual License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/v/expiration_date.svg)](https://crates.io/crates/expiration_date)
[![Downloads](https://img.shields.io/crates/d/expiration_date.svg)](https://crates.io/crates/expiration_date)
[![Stars](https://img.shields.io/github/stars/joaquinbejar/ExpirationDate.svg)](https://github.com/joaquinbejar/ExpirationDate/stargazers)
[![Issues](https://img.shields.io/github/issues/joaquinbejar/ExpirationDate.svg)](https://github.com/joaquinbejar/ExpirationDate/issues)
[![PRs](https://img.shields.io/github/issues-pr/joaquinbejar/ExpirationDate.svg)](https://github.com/joaquinbejar/ExpirationDate/pulls)
[![Build Status](https://img.shields.io/github/workflow/status/joaquinbejar/ExpirationDate/CI)](https://github.com/joaquinbejar/ExpirationDate/actions)
[![Coverage](https://img.shields.io/codecov/c/github/joaquinbejar/ExpirationDate)](https://codecov.io/gh/joaquinbejar/ExpirationDate)
[![Dependencies](https://img.shields.io/librariesio/github/joaquinbejar/ExpirationDate)](https://libraries.io/github/joaquinbejar/ExpirationDate)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/expiration_date)
[![Wiki](https://img.shields.io/badge/wiki-latest-blue.svg)](https://deepwiki.com/joaquinbejar/ExpirationDate)


## ExpirationDate

A standalone crate for handling financial instrument expiration dates in Rust.

### Overview

`ExpirationDate` is a Rust library that provides a flexible enum for representing
expiration dates of financial instruments such as options, futures, and other derivatives.
It supports two representations:

- **Days**: A positive number of days from the current date using [`Positive`](https://crates.io/crates/positive)
- **DateTime**: An absolute point in time using `chrono::DateTime<Utc>`

This is particularly useful in quantitative finance applications where expiration dates
need to be expressed either as days-to-expiration (DTE) or as specific calendar dates.

### Minimum Supported Rust Version

`expiration_date` requires **Rust 1.85** or later (Edition 2024).

### Features

- **Dual Representation**: Express expirations as days remaining or absolute datetimes
- **Time-to-Expiration**: Calculate expiration in fractional years for pricing models
- **Multiple Parse Formats**: Parse dates from RFC3339, YYYYMMDD, DD-MM-YYYY, DD-Mon-YYYY, and more
- **Prelude Module**: Simple imports with `use expiration_date::prelude::*;`
- **Serde Support**: Full serialization/deserialization support for JSON and other formats
- **Custom Ordering**: Compare and sort expirations regardless of variant type
- **Reference DateTime**: Thread-local reference time support for deterministic testing
- **Optional utoipa Integration**: OpenAPI schema generation support via feature flag

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
expiration_date = "0.1"
```

To enable OpenAPI schema support:

```toml
[dependencies]
expiration_date = { version = "0.1", features = ["utoipa"] }
```

### Quick Start

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;

// Create from days
let exp = ExpirationDate::Days(pos_or_panic!(30.0));
let years = exp.get_years().unwrap();

// Create from a specific datetime
use chrono::{TimeZone, Utc};
let dt = Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap();
let exp = ExpirationDate::DateTime(dt);

// Parse from string (supports multiple formats)
let exp = ExpirationDate::from_string("2025-12-31").unwrap();
let exp = ExpirationDate::from_string("31-dec-2025").unwrap();
let exp = ExpirationDate::from_string("20251231").unwrap();

// Get expiration as fractional years (for Black-Scholes, etc.)
let t = exp.get_years().unwrap();
```

### API Overview

#### Creation

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;
use chrono::{TimeZone, Utc};

// From days to expiration
let exp = ExpirationDate::Days(pos_or_panic!(30.0));

// From absolute datetime
let dt = Utc.with_ymd_and_hms(2025, 6, 20, 0, 0, 0).unwrap();
let exp = ExpirationDate::DateTime(dt);

// From string (auto-detects format)
let exp = ExpirationDate::from_string("2025-06-20").unwrap();
let exp = ExpirationDate::from_string("30.0").unwrap(); // Parsed as days

// Default (365 days)
let exp = ExpirationDate::default();
```

#### Time Calculations

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;

let exp = ExpirationDate::Days(pos_or_panic!(182.5));

// Get time to expiration in years (days / 365)
let years = exp.get_years().unwrap(); // ~0.5

// Get number of days as Positive
let days = exp.get_days().unwrap();

// Get the resolved DateTime<Utc>
let date = exp.get_date().unwrap();

// Get date as formatted string (YYYY-MM-DD)
let date_str = exp.get_date_string().unwrap();
```

#### String Parsing

`ExpirationDate::from_string` supports multiple date formats:

| Format | Example |
|---|---|
| Numeric (days) | `"30.0"` |
| ISO 8601 / RFC3339 | `"2025-12-31T00:00:00Z"` |
| ISO 8601 no seconds | `"2025-12-31T15:29"` |
| Date only | `"2025-12-31"` |
| YYYYMMDD | `"20251231"` |
| DD-MM-YYYY | `"31-12-2025"` |
| DD Mon YYYY | `"31 dec 2025"` |
| DD-Mon-YYYY | `"31-dec-2025"` |

#### Comparison and Ordering

Expiration dates can be compared across variant types. Mixed comparisons
normalize both sides to days for consistent ordering:

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;
use chrono::{Duration, Utc};

let thirty_days = ExpirationDate::Days(pos_or_panic!(30.0));
let sixty_days = ExpirationDate::Days(pos_or_panic!(60.0));
assert!(thirty_days < sixty_days);

// Sort a collection of mixed variants
let mut expirations = vec![sixty_days, thirty_days];
expirations.sort();
```

#### Serialization

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;

let exp = ExpirationDate::Days(pos_or_panic!(30.0));
let json = serde_json::to_string(&exp).unwrap();  // {"days":30.0}

let parsed: ExpirationDate = serde_json::from_str(&json).unwrap();
```

### Error Handling

The library provides `ExpirationDateError` for comprehensive error handling:

```rust
use expiration_date::ExpirationDate;
use expiration_date::error::ExpirationDateError;

let result = ExpirationDate::from_string("invalid date");
assert!(result.is_err());
```

Error variants include:
- `ParseError` - Failed to parse a date string
- `ConversionError` - Failed to convert between representations
- `InvalidDateTime` - Invalid datetime value encountered
- `PositiveError` - Error from the underlying `Positive` type
- `ChronoParseError` - Error from chrono date parsing
- `ParseIntError` - Error parsing integer components

### Use Cases

- **Options Pricing**: Express time-to-expiration for Black-Scholes and other models
- **Risk Management**: Track and compare expiration dates across portfolios
- **Trading Systems**: Parse expiration dates from various exchange formats
- **Financial Data**: Serialize/deserialize expiration dates in APIs and databases

### License

This project is licensed under the MIT License.




## Contribution and Contact

We welcome contributions to this project! If you would like to contribute, please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and ensure that the project still builds and all tests pass.
4. Commit your changes and push your branch to your forked repository.
5. Submit a pull request to the main repository.

If you have any questions, issues, or would like to provide feedback, please feel free to contact the project maintainer:


### **Contact Information**

- **Author**: Joaquín Béjar García
- **Email**: jb@taunais.com
- **Telegram**: [@joaquin_bejar](https://t.me/joaquin_bejar)
- **Repository**: <https://github.com/joaquinbejar/ExpirationDate>
- **Documentation**: <https://docs.rs/expiration_date>

We appreciate your interest and look forward to your contributions!

## ✍️ License

Licensed under **MIT** license
