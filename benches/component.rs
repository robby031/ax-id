use criterion::{Criterion, criterion_group, criterion_main};
use std::cell::Cell;
use std::hint::black_box;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn bench_timestamp(c: &mut Criterion) {
    let mut group = c.benchmark_group("timestamp");

    group.bench_function("systemtime_now", |b| {
        b.iter(|| {
            black_box(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            );
        });
    });

    group.bench_function("instant_now", |b| {
        b.iter(|| {
            black_box(Instant::now());
        });
    });

    group.bench_function("instant_elapsed", |b| {
        let start = Instant::now();
        b.iter(|| {
            black_box(start.elapsed().as_millis());
        });
    });

    group.finish();
}

fn bench_threadlocal(c: &mut Criterion) {
    let mut group = c.benchmark_group("threadlocal");

    group.bench_function("cell_get_set_3tuple", |b| {
        thread_local! {
            static BATCH: Cell<(u64, u64, u64)> = const { Cell::new((0, 0, 0)) };
        }
        b.iter(|| {
            let (a, b_val, c) = BATCH.with(|x| x.get());
            black_box(a);
            black_box(b_val);
            black_box(c);
            BATCH.with(|x| x.set((a + 1, b_val + 1, c + 1)));
        });
    });

    group.bench_function("cell_get_set_single_u64", |b| {
        thread_local! {
            static VAL: Cell<u64> = const { Cell::new(0) };
        }
        b.iter(|| {
            let v = VAL.with(|x| x.get());
            black_box(v);
            VAL.with(|x| x.set(v + 1));
        });
    });

    group.bench_function("cell_inline", |b| {
        thread_local! {
            static BATCH: Cell<(u64, u64, u64)> = const { Cell::new((0, 0, 0)) };
        }
        b.iter(|| {
            BATCH.with(|x| {
                let (a, b_val, c) = x.get();
                x.set((a + 1, b_val + 1, c + 1));
                black_box((a, b_val, c));
            });
        });
    });

    group.finish();
}

fn bench_axrnd(c: &mut Criterion) {
    let mut group = c.benchmark_group("axrnd");

    group.bench_function("ax_rnd_u64", |b| {
        b.iter(|| black_box(ax_rnd::u64()));
    });

    group.finish();
}

criterion_group!(benches, bench_timestamp, bench_threadlocal, bench_axrnd);
criterion_main!(benches);
