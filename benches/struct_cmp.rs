use criterion::{Criterion, criterion_group, criterion_main};
use std::cell::Cell;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CUSTOM_EPOCH: u64 = 1_704_067_200_000;
const SEQ_MASK: u64 = (1 << 13) - 1;
const MAX_SEQ: u64 = SEQ_MASK;

struct FastGen {
    state: AtomicU64,
    node_id: u64,
}

impl FastGen {
    #[inline(always)]
    fn generate_fast(&self) -> u64 {
        thread_local! {
            static BATCH: Cell<(u64, u64, u64)> = const { Cell::new((u64::MAX, 0, 0)) };
        }

        let (ts, seq, end) = BATCH.with(|b| b.get());
        if seq < end {
            BATCH.with(|b| b.set((ts, seq + 1, end)));
            return (ts << 23) | (self.node_id << 13) | seq;
        }

        // slow path: just get new timestamp and reset
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let rel = now - CUSTOM_EPOCH;
        let loaded = self.state.load(Ordering::Acquire);
        let last_ts = loaded >> 13;
        let last_seq = loaded & SEQ_MASK;

        let (new_ts, new_seq, batch_end) = if rel == last_ts {
            let end = (last_seq + 512).min(MAX_SEQ);
            (last_ts, last_seq, end)
        } else {
            (rel, 0, 512)
        };

        let new_state = (new_ts << 13) | batch_end;
        let _ =
            self.state
                .compare_exchange(loaded, new_state, Ordering::Release, Ordering::Relaxed);
        BATCH.with(|b| b.set((new_ts, new_seq + 1, batch_end)));
        (new_ts << 23) | (self.node_id << 13) | new_seq
    }

    #[inline(always)]
    fn generate(&self) -> u64 {
        thread_local! {
            static BATCH: Cell<(u64, u64, u64)> = const { Cell::new((u64::MAX, 0, 0)) };
            static LAST_CHK: Cell<Instant> = Cell::new(Instant::now());
        }

        let (ts, seq, end) = BATCH.with(|b| b.get());
        if seq < end {
            BATCH.with(|b| b.set((ts, seq + 1, end)));
            return (ts << 23) | (self.node_id << 13) | seq;
        }

        let last_chk = LAST_CHK.with(|c| c.get());
        if last_chk.elapsed().as_millis() == 0 {
            let loaded = self.state.load(Ordering::Acquire);
            let last_ts = loaded >> 13;
            let last_seq = loaded & SEQ_MASK;
            if last_seq < MAX_SEQ {
                let batch_end = (last_seq + 512).min(MAX_SEQ);
                let new_state = (last_ts << 13) | batch_end;
                if self
                    .state
                    .compare_exchange_weak(loaded, new_state, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    BATCH.with(|b| b.set((last_ts, last_seq + 1, batch_end)));
                    return (last_ts << 23) | (self.node_id << 13) | last_seq;
                }
            }
        }

        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let rel = now - CUSTOM_EPOCH;
            let loaded = self.state.load(Ordering::Acquire);
            let last_ts = loaded >> 13;
            let last_seq = loaded & SEQ_MASK;

            let (new_ts, new_seq, batch_end) = if rel == last_ts {
                if last_seq >= MAX_SEQ {
                    std::hint::spin_loop();
                    continue;
                }
                let end = (last_seq + 512).min(MAX_SEQ);
                (last_ts, last_seq, end)
            } else if rel > last_ts {
                (rel, 0, 512)
            } else {
                std::thread::yield_now();
                continue;
            };

            let new_state = (new_ts << 13) | batch_end;
            match self.state.compare_exchange(
                loaded,
                new_state,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    LAST_CHK.with(|c| c.set(Instant::now()));
                    BATCH.with(|b| b.set((new_ts, new_seq + 1, batch_end)));
                    return (new_ts << 23) | (self.node_id << 13) | new_seq;
                }
                Err(_) => {
                    std::hint::spin_loop();
                    continue;
                }
            }
        }
    }
}

fn bench_struct(c: &mut Criterion) {
    let mut group = c.benchmark_group("struct_cmp");

    group.bench_function("ax_id_generator", |b| {
        let generator = ax_id::Generator::new(1).unwrap();
        b.iter(|| black_box(generator.generate_simple()));
    });

    group.bench_function("fast_gen_inline", |b| {
        let generator = FastGen {
            state: AtomicU64::new(0),
            node_id: 1,
        };
        b.iter(|| black_box(generator.generate()));
    });

    group.bench_function("fast_gen_fast", |b| {
        let generator = FastGen {
            state: AtomicU64::new(0),
            node_id: 1,
        };
        b.iter(|| black_box(generator.generate_fast()));
    });

    group.finish();
}

criterion_group!(benches, bench_struct);
criterion_main!(benches);
