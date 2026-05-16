use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::IdError;
use crate::id::Id;
use crate::internal::{
    MAX_NODE_ID, MAX_SEQUENCE, NODE_SHIFT, SEQUENCE_BITS, SEQUENCE_MASK, TIMESTAMP_SHIFT,
    current_timestamp_ms, resolve_node_id, thread_local_entropy,
};

const BATCH_SIZE: u64 = 4096;

#[derive(Debug)]
pub struct Generator {
    state: AtomicU64,
    pub(crate) node_id: u16,
    instance_tag: u64,
}

impl Generator {
    #[inline]
    pub const fn node_id(&self) -> u16 {
        self.node_id
    }

    pub fn new(node_id: u16) -> Result<Self, IdError> {
        if node_id > MAX_NODE_ID {
            return Err(IdError::InvalidNodeId(node_id));
        }
        let ts = current_timestamp_ms()?;
        let state = ts << SEQUENCE_BITS;
        Ok(Self {
            state: AtomicU64::new(state),
            node_id,
            instance_tag: thread_local_entropy(),
        })
    }

    #[doc(hidden)]
    pub fn with_timestamp(node_id: u16, timestamp_ms: u64) -> Result<Self, IdError> {
        if node_id > MAX_NODE_ID {
            return Err(IdError::InvalidNodeId(node_id));
        }
        if timestamp_ms >= (1u64 << crate::internal::TIMESTAMP_BITS) {
            return Err(IdError::ClockSkew {
                elapsed_ms: timestamp_ms,
            });
        }
        let state = timestamp_ms << SEQUENCE_BITS;
        Ok(Self {
            state: AtomicU64::new(state),
            node_id,
            instance_tag: thread_local_entropy(),
        })
    }

    pub fn new_auto() -> Self {
        let node_id = resolve_node_id();
        Self::new(node_id).expect("auto-resolved node_id is always valid")
    }

    #[inline(always)]
    pub fn generate(&self) -> Result<Id, IdError> {
        use std::cell::Cell;

        thread_local! {
            static BATCH: Cell<(u64, u64, u64, u64)> = const { Cell::new((0, u64::MAX, 0, 0)) };
        }

        let (tag, ts, seq, end) = BATCH.with(|b| b.get());
        if tag == self.instance_tag && ts != u64::MAX && seq < end {
            BATCH.with(|b| b.set((tag, ts, seq + 1, end)));
            let id = (ts << TIMESTAMP_SHIFT) | ((self.node_id as u64) << NODE_SHIFT) | seq;
            return Ok(Id(id));
        }

        let mut now = current_timestamp_ms()?;
        loop {
            let loaded = self.state.load(Ordering::Acquire);
            let last_ts = loaded >> SEQUENCE_BITS;
            let last_seq = loaded & SEQUENCE_MASK;

            let (new_ts, new_seq, batch_end) = if now == last_ts {
                if last_seq >= MAX_SEQUENCE {
                    std::hint::spin_loop();
                    now = current_timestamp_ms()?;
                    continue;
                }
                let end = (last_seq + BATCH_SIZE).min(MAX_SEQUENCE);
                (last_ts, last_seq, end)
            } else if now > last_ts {
                let end = BATCH_SIZE.min(MAX_SEQUENCE);
                (now, 0, end)
            } else {
                let elapsed = last_ts - now;
                if elapsed <= 1000 {
                    std::thread::yield_now();
                    now = current_timestamp_ms()?;
                    continue;
                }
                return Err(IdError::ClockSkew {
                    elapsed_ms: elapsed,
                });
            };

            let new_state = (new_ts << SEQUENCE_BITS) | batch_end;
            match self.state.compare_exchange(
                loaded,
                new_state,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    BATCH.with(|b| b.set((self.instance_tag, new_ts, new_seq + 1, batch_end)));
                    let id = (new_ts << TIMESTAMP_SHIFT)
                        | ((self.node_id as u64) << NODE_SHIFT)
                        | new_seq;
                    return Ok(Id(id));
                }
                Err(_) => {
                    std::hint::spin_loop();
                    now = current_timestamp_ms()?;
                    continue;
                }
            }
        }
    }

    #[inline(always)]
    pub fn generate_simple(&self) -> Id {
        use std::cell::Cell;

        thread_local! {
            static TS: Cell<u64> = const { Cell::new(0) };
            static SEQ: Cell<u64> = const { Cell::new(u64::MAX) };
        }

        let ts = TS.with(|c| c.get());
        let seq = SEQ.with(|c| c.get());

        if seq <= MAX_SEQUENCE {
            SEQ.with(|c| c.set(seq + 1));
            return Id((ts << TIMESTAMP_SHIFT) | ((self.node_id as u64) << NODE_SHIFT) | seq);
        }

        loop {
            let now = match current_timestamp_ms() {
                Ok(t) => t,
                Err(_) => {
                    std::thread::yield_now();
                    continue;
                }
            };

            if now > ts {
                TS.with(|c| c.set(now));
                SEQ.with(|c| c.set(1));
                return Id((now << TIMESTAMP_SHIFT) | ((self.node_id as u64) << NODE_SHIFT));
            }
            std::hint::spin_loop();
        }
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new_auto()
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use alloc::collections::BTreeSet;

    // Independent generators with different node IDs must be globally unique.
    #[test]
    fn cross_generator_uniqueness_same_thread() {
        let g1 = Generator::new(1).unwrap();
        let g2 = Generator::new(2).unwrap();
        let mut set = BTreeSet::new();

        for _ in 0..100_000 {
            let id1 = g1.generate().unwrap();
            let id2 = g2.generate().unwrap();
            assert!(set.insert(id1.0), "duplicate from g1");
            assert!(set.insert(id2.0), "duplicate from g2");
        }
    }

    // Stale-batch guard: creating a new generator on the same thread must not
    // accidentally reuse the old generator's thread-local batch.
    #[test]
    fn stale_batch_guard_same_thread() {
        let g1 = Generator::new(1).unwrap();

        let _ = g1.generate().unwrap();

        let g2 = Generator::new(1).unwrap();
        let id2_first = g2.generate().unwrap();

        assert_eq!(id2_first.raw_sequence(), 0);
    }

    // High-volume atomic generation — 1 million IDs from a single generator.
    #[test]
    fn high_volume_atomic() {
        let g = Generator::new(1).unwrap();
        let mut set = BTreeSet::new();
        for _ in 0..1_000_000 {
            let id = g.generate().unwrap();
            assert!(set.insert(id.0), "duplicate in high-volume atomic");
        }
    }

    // High-volume simple generation — 1 million IDs from a single generator.
    #[test]
    fn high_volume_simple() {
        let g = Generator::new(1).unwrap();
        let mut set = BTreeSet::new();
        for _ in 0..1_000_000 {
            let id = g.generate_simple();
            assert!(set.insert(id.0), "duplicate in high-volume simple");
        }
    }

    // Multi-threaded stress test with shared generator.
    #[test]
    fn multi_thread_stress() {
        use std::thread;
        let g = std::sync::Arc::new(Generator::new(1).unwrap());
        let mut handles = Vec::new();
        let per_thread = 100_000;

        for _ in 0..8 {
            let arc_gen = std::sync::Arc::clone(&g);
            handles.push(thread::spawn(move || {
                let mut local = BTreeSet::new();
                for _ in 0..per_thread {
                    let id = arc_gen.generate().unwrap();
                    assert!(local.insert(id.0), "duplicate in thread");
                }
                local
            }));
        }

        let mut global = BTreeSet::new();
        for h in handles {
            for raw in h.join().unwrap() {
                assert!(global.insert(raw), "duplicate across threads");
            }
        }
    }
}
