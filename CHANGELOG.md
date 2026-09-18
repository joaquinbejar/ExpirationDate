# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2026-09-18

### Changed
- Dependencies updated to latest stable versions (rust_decimal 1.42 -> 1.43)

## [0.3.0] - 2026-08-17

### Changed — breaking
- `positive` bumped from 0.5 to 0.6. `Positive` appears throughout this
  crate's public API (`ExpirationDate::Days`,
  `ExpirationDateError::PositiveError`), so consumers must move to
  `positive` 0.6 in the same step. See the `positive` 0.6.0 changelog for
  its own breaking changes (`ln`/`log10` return `Decimal`, serde emits a
  string, `==` against `Decimal`/`f64` is exact, `PositiveError::Other`
  and `new_unchecked` removed).
- `get_date`, `get_date_with_base` and `get_date_with_options` return
  `Err(ExpirationDateError::PositiveError(..))` instead of panicking when
  `Days` holds a value above `i64::MAX`: the deprecated
  `Positive::to_i64` was replaced by `i64::try_from`.

### Housekeeping
- Folds in 0.2.1, which was released without a changelog entry of its own.
- `.cargo/audit.toml`, mirroring the `positive` crate's policy file: it ignores
  RUSTSEC-2026-0235 (rkyv 0.7.46) with a reachability rationale — `rkyv` is an
  *optional* dependency of `rust_decimal` that this crate never enables, so it
  is recorded in `Cargo.lock` but never compiled (`cargo tree --all-features
  --target all -i rkyv` reports nothing) — and opts into failing on
  unmaintained/unsound/notice advisories. The entry has an owner and a
  2027-02-15 review date.

## [0.2.0] - 2026-04-15

### Added
- Day count conventions module (`Actual360`, `Actual365Fixed`, `Thirty360US`) and `get_years_with_convention` API.
- `ArithmeticOverflow(String)` variant on `ExpirationDateError` for checked arithmetic in conventions.
- Thread-local `REFERENCE_DATETIME` with `set_reference_datetime` / `get_reference_datetime` accessors for deterministic testing.
- Comprehensive integration test suite under `tests/` (70 tests across 6 concern-focused files) plus 5 proptest properties (1280 cases per run).
- Criterion benchmarks for parser, conversion, and serde hot paths under `benches/`.
- `CHANGELOG.md` (Keep a Changelog 1.1.0).
- `.github/dependabot.yml` for cargo + github-actions weekly grouped updates.
- `rust-version = "1.85"` MSRV declared in `Cargo.toml`.
- `[package.metadata.docs.rs]` with `all-features=true` and `docsrs` cfg.
- Module split: `src/{lib,parser,serde_impl,cmp,convert,error,prelude,conventions}.rs` for clearer review and faster incremental compile.
- `make publish-dry` and `make publish` (with confirmation prompt) replace the misleading single `publish` target.
- `make bench` target.

### Changed
- `src/lib.rs` reduced from 502 to ~65 lines (enum + EPSILON + thread-local reference datetime).
- Error messages lowercased to match project convention.
- `Ord` impl uses 4-arm pattern to preserve antisymmetry when both sides error.
- `Deserialize` rejects duplicate `days` / `datetime` fields explicitly.
- Makefile project header references `expiration_date` (was a copy-pasted "OTC RFQ Engine").

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

[Unreleased]: https://github.com/joaquinbejar/ExpirationDate/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.3.1
[0.3.0]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.3.0
[0.2.0]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.2.0
[0.1.2]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.2
[0.1.1]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.1
[0.1.0]: https://github.com/joaquinbejar/ExpirationDate/releases/tag/v0.1.0
