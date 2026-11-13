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
- **Clock-free Builds**: Pure conversions that take the time as an argument; disable the `clock` feature and nothing can read the wall clock
- **Reference DateTime**: Thread-local reference time support for deterministic testing
- **Optional utoipa Integration**: OpenAPI schema generation support via feature flag

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
expiration_date = "0.6"
```

To enable OpenAPI schema support:

```toml
[dependencies]
expiration_date = { version = "0.6", features = ["utoipa"] }
```

To guarantee that the crate cannot read the wall clock, for a simulator or a
replay engine, turn off the `clock` feature, which is on by default:

```toml
[dependencies]
expiration_date = { version = "0.6", default-features = false }
```

Cargo unifies features across a build, so check that no other crate turns it
back on: `cargo tree -e features -i chrono` must not list `clock` or `now`.

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

These methods ask the system for the time and need the `clock` feature. Each
has a pure form that takes the time as an argument, ignores the reference
datetime and exists in every build:

| Reads the clock | Pure |
|-----------------|------|
| `get_years()` | `get_years_at(now)` |
| `get_years_with_convention(c)` | `get_years_with_convention_at(c, now)` |
| `get_days()` | `get_days_at(now)` |
| `get_date()`, `get_date_with_options(false)` | `get_date_at(base)` |
| `get_date_string()` | `get_date_string_at(today)` |
| `from_string_to_days(s)` | `from_string_to_days_at(s, now)` |

```rust
use chrono::{Duration, TimeZone, Utc};
use expiration_date::ExpirationDate;

let now = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
let exp = ExpirationDate::DateTime(now + Duration::days(365));

let years = exp.get_years_at(now).unwrap(); // 1.0
```

`Display` exists in every build. A `DateTime` prints its instant. A `Days`
prints the instant it resolves to from the reference datetime; with none set it
reads the clock when the `clock` feature is on, and otherwise prints the count,
as in `30 days`.

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

Equality, hashing and ordering never read the clock or the reference datetime,
so a sorted collection or a hash map stays valid as time passes.

Two `DateTime`s are equal when they are the same instant, two `Days` when their
day counts are equal, and a `Days` never equals a `DateTime`. `a == b` implies
equal hashes, and `Ord` is total: within a variant it is chronological, and
every `Days` comes before every `DateTime`. Past dates keep their own instants,
so two expired expiries stay distinct and ordered.

That order between variants says nothing about which one expires first, which
depends on a base. To compare in time, pass one to `cmp_at`:

```rust
use expiration_date::ExpirationDate;
use positive::pos_or_panic;
use chrono::{Duration, TimeZone, Utc};
use std::cmp::Ordering;

let thirty_days = ExpirationDate::Days(pos_or_panic!(30.0));
let sixty_days = ExpirationDate::Days(pos_or_panic!(60.0));
assert!(thirty_days < sixty_days);

// Sort a collection
let mut expirations = vec![sixty_days, thirty_days];
expirations.sort();

// Compare across variants in time, from an explicit base
let base = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
let in_ten_days = ExpirationDate::DateTime(base + Duration::days(10));
assert_eq!(thirty_days.cmp_at(&in_ten_days, base), Ordering::Greater);
assert!(thirty_days < in_ten_days); // `Ord` orders every `Days` first
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
- `ArithmeticOverflow` - A calculation does not fit its type, such as a day count past the last representable date

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

### Development

- Run tests: `make test`
- Lint: `make lint`
- Format: `make fmt`
- Run benchmarks: `make bench` (wraps `cargo bench --all-features`; Criterion benches live under `benches/`)

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

<!-- related-projects:start -->
## Related projects

Repositories by the same author that this project depends on, and repositories that depend on it.

### Depends on

| Repository | Description |
|------------|-------------|
| [positive](https://github.com/joaquinbejar/positive) | Provides the `Positive` type used for day counts and year fractions. |

### Used by

| Repository | Description |
|------------|-------------|
| [OptionStratLib](https://github.com/joaquinbejar/OptionStratLib) | Options trading and strategy library; uses ExpirationDate for every option's expiry. |
| [option_type](https://github.com/joaquinbejar/option_type) | Enum-based classification of vanilla and exotic options; uses ExpirationDate for option expiries. |

<!-- related-projects:end -->
