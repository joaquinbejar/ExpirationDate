//! Benchmarks for the hand-written serde impls on [`ExpirationDate`].
//!
//! The tagged-map wire shape (`{"days": N}` / `{"datetime": "..."}`) is
//! a semver contract, so serialization and deserialization speed matter
//! for any caller persisting or reading these values in bulk.

#![allow(missing_docs)]
#![allow(clippy::expect_used)]

use chrono::{TimeZone, Utc};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use expiration_date::ExpirationDate;

fn build_days_variant() -> ExpirationDate {
    let days = "30.0".parse::<positive::Positive>().expect("test fixture");
    ExpirationDate::Days(days)
}

fn build_datetime_variant() -> ExpirationDate {
    let dt = Utc
        .with_ymd_and_hms(2025, 12, 31, 18, 30, 0)
        .single()
        .expect("test fixture");
    ExpirationDate::DateTime(dt)
}

fn bench_serialize(c: &mut Criterion) {
    let days_variant = build_days_variant();
    let dt_variant = build_datetime_variant();

    c.bench_function("serde/to_string/days_variant", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::to_string(black_box(&days_variant)));
        });
    });

    c.bench_function("serde/to_string/datetime_variant", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::to_string(black_box(&dt_variant)));
        });
    });
}

fn bench_deserialize(c: &mut Criterion) {
    let days_variant = build_days_variant();
    let dt_variant = build_datetime_variant();
    let days_json = serde_json::to_string(&days_variant).expect("test fixture");
    let dt_json = serde_json::to_string(&dt_variant).expect("test fixture");

    c.bench_function("serde/from_str/days_variant", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::from_str::<ExpirationDate>(black_box(
                days_json.as_str(),
            )));
        });
    });

    c.bench_function("serde/from_str/datetime_variant", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::from_str::<ExpirationDate>(black_box(
                dt_json.as_str(),
            )));
        });
    });
}

criterion_group!(benches, bench_serialize, bench_deserialize);
criterion_main!(benches);
