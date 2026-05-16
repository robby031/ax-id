use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse/hex", |b| {
        b.iter(|| {
            black_box("08b53edc41582000".parse::<ax_id::Id>().unwrap());
        });
    });

    c.bench_function("parse/decimal", |b| {
        b.iter(|| {
            black_box("627480090874970112".parse::<ax_id::Id>().unwrap());
        });
    });
}

fn bench_format(c: &mut Criterion) {
    let id = ax_id::Id(0x08b53edc41582000);
    c.bench_function("format/display", |b| {
        b.iter(|| {
            black_box(id.to_string());
        });
    });
}

#[cfg(feature = "serde")]
fn bench_serde(c: &mut Criterion) {
    let id = ax_id::Id(0x08b53edc41582000);
    c.bench_function("serde/json/serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&id).unwrap());
        });
    });

    let json = serde_json::to_string(&id).unwrap();
    c.bench_function("serde/json/deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<ax_id::Id>(&json).unwrap());
        });
    });
}

#[cfg(not(feature = "serde"))]
fn bench_serde(_c: &mut Criterion) {}

fn bench_hex(c: &mut Criterion) {
    let id = ax_id::Id(0x08b53edc41582000);
    c.bench_function("hex/from_str", |b| {
        b.iter(|| {
            black_box("08b53edc41582000".parse::<ax_id::Id>().unwrap());
        });
    });

    c.bench_function("hex/format", |b| {
        b.iter(|| {
            black_box(format!("{:016x}", id.0));
        });
    });
}

criterion_group!(benches, bench_parse, bench_format, bench_serde, bench_hex);
criterion_main!(benches);
