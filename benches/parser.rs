//! Benchmarks for [`ExpirationDate::from_string`] covering every input
//! shape the parser advertises in its docs.
//!
//! Compile only via `cargo bench --no-run`; run locally with
//! `cargo bench --bench parser` when measuring changes. Every input is
//! wrapped in [`black_box`] so the optimizer cannot elide the parse.

#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use expiration_date::ExpirationDate;

fn bench_from_string(c: &mut Criterion) {
    c.bench_function("from_string/iso_date", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box("2025-12-31")));
        });
    });

    c.bench_function("from_string/dd_mm_yyyy", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box("31-12-2025")));
        });
    });

    // Note: "20251231" actually parses as Days(20251231.0) because the
    // parser tries `Positive` first. The benchmark still exercises the
    // numeric fast path which is part of the hot surface.
    c.bench_function("from_string/yyyymmdd_numeric", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box("20251231")));
        });
    });

    c.bench_function("from_string/rfc3339", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box(
                "2025-12-31T18:30:00Z",
            )));
        });
    });

    c.bench_function("from_string/utc_suffix", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box(
                "2025-12-31 18:30:00 UTC",
            )));
        });
    });

    c.bench_function("from_string/numeric_days", |b| {
        b.iter(|| {
            let _ = black_box(ExpirationDate::from_string(black_box("30.0")));
        });
    });
}

criterion_group!(benches, bench_from_string);
criterion_main!(benches);
