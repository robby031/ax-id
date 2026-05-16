use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_local_generator(c: &mut Criterion) {
    let mut group = c.benchmark_group("local_generator");

    group.bench_function("ax_id_shared", |b| {
        let generator = ax_id::Generator::new(1).unwrap();
        b.iter(|| black_box(generator.generate_simple()));
    });

    group.bench_function("ax_id_local_noatomic", |b| {
        use std::cell::Cell;
        use std::time::{SystemTime, UNIX_EPOCH};

        const CUSTOM_EPOCH: u64 = 1_704_067_200_000;
        const SEQ_MASK: u64 = (1 << 13) - 1;

        thread_local! {
            static TS: Cell<u64> = const { Cell::new(0) };
            static SEQ: Cell<u64> = const { Cell::new(0) };
        }

        b.iter(|| {
            let ts = TS.with(|c| {
                let cached = c.get();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let rel = now - CUSTOM_EPOCH;
                if rel != cached {
                    c.set(rel);
                    SEQ.with(|s| s.set(0));
                }
                rel
            });
            let seq = SEQ.with(|c| {
                let s = c.get() + 1;
                c.set(s);
                s
            });
            let id = (ts << 23) | (1u64 << 13) | (seq & SEQ_MASK);
            black_box(id);
        });
    });

    group.bench_function("ax_id_local_cached_ts", |b| {
        use std::cell::Cell;
        use std::time::{Instant, SystemTime, UNIX_EPOCH};

        const CUSTOM_EPOCH: u64 = 1_704_067_200_000;
        const SEQ_MASK: u64 = (1 << 13) - 1;

        thread_local! {
            static TS: Cell<u64> = const { Cell::new(0) };
            static SEQ: Cell<u64> = const { Cell::new(0) };
            static LAST_CHECK: Cell<Instant> = Cell::new(Instant::now());
        }

        b.iter(|| {
            let ts = TS.with(|c| {
                let cached = c.get();
                let last = LAST_CHECK.with(|l| l.get());
                if last.elapsed().as_millis() == 0 {
                    return cached;
                }
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let rel = now - CUSTOM_EPOCH;
                if rel != cached {
                    c.set(rel);
                    SEQ.with(|s| s.set(0));
                }
                LAST_CHECK.with(|l| l.set(Instant::now()));
                rel
            });
            let seq = SEQ.with(|c| {
                let s = c.get() + 1;
                c.set(s);
                s
            });
            let id = (ts << 23) | (1u64 << 13) | (seq & SEQ_MASK);
            black_box(id);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_local_generator);
criterion_main!(benches);
