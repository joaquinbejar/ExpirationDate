//! Benchmarks for the day / year accessors on [`ExpirationDate`].
//!
//! `get_days` and `get_years` sit on the pricing hot path, so this bench
//! exercises both the `Days` fast path (no `Utc::now` call) and the
//! `DateTime` variant which pays for a wall-clock read plus the
//! Actual/365 fixed year fraction.

#![allow(missing_docs)]
#![allow(clippy::expect_used)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use expiration_date::ExpirationDate;

fn build_datetime_variant() -> ExpirationDate {
    let dt = chrono::Utc::now() + chrono::Duration::days(30);
    ExpirationDate::DateTime(dt)
}

fn build_days_variant() -> ExpirationDate {
    let days = "30.0".parse::<positive::Positive>().expect("test fixture");
    ExpirationDate::Days(days)
}

fn bench_get_days(c: &mut Criterion) {
    let days_variant = build_days_variant();
    let dt_variant = build_datetime_variant();

    c.bench_function("get_days/days_variant", |b| {
        b.iter(|| {
            let _ = black_box(black_box(&days_variant).get_days());
        });
    });

    c.bench_function("get_days/datetime_variant", |b| {
        b.iter(|| {
            let _ = black_box(black_box(&dt_variant).get_days());
        });
    });
}

fn bench_get_years(c: &mut Criterion) {
    let days_variant = build_days_variant();
    let dt_variant = build_datetime_variant();

    c.bench_function("get_years/days_variant", |b| {
        b.iter(|| {
            let _ = black_box(black_box(&days_variant).get_years());
        });
    });

    c.bench_function("get_years/datetime_variant", |b| {
        b.iter(|| {
            let _ = black_box(black_box(&dt_variant).get_years());
        });
    });
}

criterion_group!(benches, bench_get_days, bench_get_years);
criterion_main!(benches);
