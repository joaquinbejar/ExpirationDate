# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Day count conventions module (`Actual360`, `Actual365Fixed`, `Thirty360US`) and `get_years_with_convention` API.
- `ArithmeticOverflow(String)` variant on `ExpirationDateError` for checked arithmetic in conventions.
- Comprehensive integration test suite under `tests/` (70 tests across 6 concern-focused files).
- Module split: `src/{lib,parser,serde_impl,cmp,convert,error,prelude,conventions}.rs` for clearer review and faster incremental compile.

### Changed
- `src/lib.rs` reduced from 502 to ~65 lines (enum + EPSILON + thread-local reference datetime).
- Error messages lowercased to match project convention.
- `Ord` impl uses 4-arm pattern to preserve antisymmetry when both sides error.
- `Deserialize` rejects duplicate `days` / `datetime` fields explicitly.

### Fixed
- Restored parse formats lost during early v0.2.0 work: `%Y-%m-%d %H:%M:%S %Z`, `... UTC`, `T15:29` (no seconds), `%d %B %Y`, `%d-%B-%Y`.
- Restored case-insensitive month parsing (`.to_lowercase()`).
- Restored `set_reference_datetime` side-effect on `from_string` and `get_days(DateTime)`.
- Restored runnable `# Examples` doctests on `get_years`, `get_date`, `get_date_string`, `from_string`.

## [0.1.2] - 2026-04-15

### Changed
- Bumped `positive` dependency from `0.4` to `0.5`.
- `PartialEq` switched from private tuple field access (`Positive.0`) to the public `to_dec()` accessor.
- Refreshed other dependency versions.

## [0.1.1] - 2026-02-19

### Added
- Comprehensive `README.md` with usage examples, features, and contribution details.
- `prelude` module re-exporting common items.
- `must_use` attributes on the public API surface.

### Changed
- Hardened expiration comparisons.
- Refactored constants usage.

## [0.1.0] - Initial release

### Added
- `ExpirationDate` enum with `Days(Positive)` and `DateTime(DateTime<Utc>)` variants.
- `from_string` parser supporting RFC3339, `YYYYMMDD`, `DD-MM-YYYY`, and numeric days.
- Hand-written `Serialize` / `Deserialize` with strict field validation.
- Hand-written `PartialEq` / `Eq` / `PartialOrd` / `Ord` / `Hash` with `EPSILON` tolerance.
- Optional `utoipa` feature for OpenAPI schema generation.

[Unreleased]: https://github.com/joaquinbejar/ExpirationDate/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.2
[0.1.1]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.1
[0.1.0]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.0
