use criterion::{Criterion, criterion_group, criterion_main};
use std::cell::Cell;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CUSTOM_EPOCH: u64 = 1_704_067_200_000;
const SEQ_MASK: u64 = (1 << 13) - 1;
const MAX_SEQ: u64 = SEQ_MASK;

fn bench_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug");

    // Baseline: current ax-id Generator
    group.bench_function("ax_id_full", |b| {
        let generator = ax_id::Generator::new(1).unwrap();
        b.iter(|| black_box(generator.generate_simple()));
    });

    // No Result wrapper, direct return
    group.bench_function("no_result", |b| {
        thread_local! {
            static BATCH: Cell<(u64, u64, u64)> = const { Cell::new((u64::MAX, 0, 0)) };
        }
        let state = AtomicU64::new(0);
        let node_id: u64 = 1;
        b.iter(|| {
            let (ts, seq, end) = BATCH.with(|b| b.get());
            if seq < end {
                BATCH.with(|b| b.set((ts, seq + 1, end)));
                let id = (ts << 23) | (node_id << 13) | seq;
                black_box(id);
                return;
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let rel = now - CUSTOM_EPOCH;
            let loaded = state.load(Ordering::Acquire);
            let last_ts = loaded >> 13;
            let last_seq = loaded & SEQ_MASK;
            let (new_ts, new_seq, batch_end) = if rel == last_ts {
                let end = (last_seq + 512).min(MAX_SEQ);
                (last_ts, last_seq, end)
            } else {
                (rel, 0, 512)
            };
            let new_state = (new_ts << 13) | batch_end;
            if state
                .compare_exchange_weak(loaded, new_state, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                BATCH.with(|b| b.set((new_ts, new_seq + 1, batch_end)));
                let id = (new_ts << 23) | (node_id << 13) | new_seq;
                black_box(id);
            }
        });
    });

    // No atomic at all (single-thread only, for comparison)
    group.bench_function("no_atomic", |b| {
        thread_local! {
            static TS: Cell<u64> = const { Cell::new(u64::MAX) };
            static SEQ: Cell<u64> = const { Cell::new(0) };
            static LAST: Cell<Instant> = Cell::new(Instant::now());
        }
        let node_id: u64 = 1;
        b.iter(|| {
            let ts = TS.with(|c| c.get());
            let seq = SEQ.with(|c| c.get());
            if ts != u64::MAX && seq < MAX_SEQ {
                SEQ.with(|c| c.set(seq + 1));
                let id = (ts << 23) | (node_id << 13) | seq;
                black_box(id);
                return;
            }
            let last = LAST.with(|c| c.get());
            if last.elapsed().as_millis() == 0 && ts != u64::MAX {
                SEQ.with(|c| c.set(seq + 1));
                let id = (ts << 23) | (node_id << 13) | seq;
                black_box(id);
                return;
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let rel = now - CUSTOM_EPOCH;
            TS.with(|c| c.set(rel));
            SEQ.with(|c| c.set(1));
            LAST.with(|c| c.set(Instant::now()));
            let id = (rel << 23) | (node_id << 13);
            black_box(id);
        });
    });

    // Minimal: just timestamp + bit packing, no thread-local, no atomic
    group.bench_function("minimal", |b| {
        let node_id: u64 = 1;
        let mut seq = 0u64;
        b.iter(|| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let rel = now - CUSTOM_EPOCH;
            seq = (seq + 1) & SEQ_MASK;
            let id = (rel << 23) | (node_id << 13) | seq;
            black_box(id);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_variants);
criterion_main!(benches);
