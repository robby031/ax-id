# ax-id Design Document

Formal specification of the 64-bit ID format, generation algorithm, and operational guarantees.

---

## 1. Bit Layout

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0|                         timestamp (40)                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         node id (10)          |        sequence (13)          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

63  62                                          22 21        13 12       0
| 0 | <----------- timestamp (40) -------------> | <- node -> | <- seq -> |
```

- **timestamp**: milliseconds since `CUSTOM_EPOCH_MS` (2024-01-01T00:00:00Z)
- **node id**: 0–1023, identifies the generating process/node
- **sequence**: counter within a millisecond, 0–8191

## 2. Limits

| Field | Bits | Range | Overflow Behavior |
|-------|------|-------|-------------------|
| Timestamp | 40 | ~34 years (2024–2058) | Silent wrap (see §4) |
| Node ID | 10 | 0–1023 | Rejected at `Generator::new()` |
| Sequence | 13 | 0–8191 | Spin/yield until next ms (see §3) |
| Total IDs/node | — | 8,192/ms ≈ 8.2×10⁹/day | — |
| Total IDs/cluster | — | 8,192×1024/ms ≈ 8.4×10¹²/day | — |

## 3. Rollback & Clock Skew Policy

### Detection
`current_timestamp_ms()` returns `Err(IdError::ClockSkew)` when:
1. `SystemTime::now()` is before UNIX_EPOCH
2. Result is before `CUSTOM_EPOCH_MS`

### Handling in `generate()`
```
if now < last_ts:
    elapsed = last_ts - now
    if elapsed <= 1000 ms:
        std::thread::yield_now()   // transient NTP jitter / time sync
        retry
    else:
        return Err(ClockSkew)      # significant clock rollback (>1s)
```

### Handling in `generate_simple()`
```
loop:
    match current_timestamp_ms():
        Ok(now) -> if now > last_ts: advance; else spin
        Err(_)  -> std::thread::yield_now()   // block until stable
```

`generate_simple()` only advances forward. A backward clock causes it to
spin until the clock catches up; it never emits an ID with a timestamp
older than the previously emitted one.

**Guarantee:** Both `generate()` and `generate_simple()` are monotonic per
thread. `generate()` either waits (≤1000ms skew) or returns
`Err(ClockSkew)` for larger skews; `generate_simple()` blocks
indefinitely (the path is infallible by design).

## 4. Overflow Behavior

### Sequence Exhaustion (per millisecond)
When 8192 IDs are emitted in the same millisecond on the same node:

- `generate()` → spin-loop (`std::hint::spin_loop()`) until `current_timestamp_ms()` advances
- `generate_simple()` → spin-loop until the clock advances; it does **not**
  fall back to `generate()` because the two paths manage independent
  sequence state and could produce overlapping sequence numbers in the
  same millisecond.

**Guarantee:** No sequence wrap. IDs remain strictly unique per node.

### Timestamp Exhaustion (~2058)
The 40-bit timestamp field wraps after ~34 years. There is **no explicit overflow handling**.

After wrap:
- IDs become sortable only within the 34-year window
- Collision risk if old archived IDs coexist with new wrapped IDs

**Mitigation:** Monitor `timestamp_ms()` in production; migrate to a new format before 2058.

## 5. Thread Correctness

### Invariant
> For any single `Generator` instance, every call to `generate()` returns a unique `Id`.

### Mechanism

**Atomic state:** `AtomicU64` stores `(timestamp << 13) | sequence_end_of_batch`.

**Instance isolation:** Each `Generator` carries a random `instance_tag: u64` assigned at construction. Thread-local `BATCH` and `LAST_CHK` are tagged with this value. A fast-path hit is only accepted when `BATCH.tag == self.instance_tag`, preventing a newly created generator on the same thread from accidentally consuming a stale batch left by a previous generator instance.

**Fast path (thread-local batch):**
```
BATCH = (instance_tag, last_timestamp, next_sequence, batch_end)
if tag matches && next_sequence < batch_end:
    return ID without atomic operation
```

**Slow path (atomic reservation):**
```
CAS loop:
    read atomic state
    reserve next 512 sequence numbers (or remainder to 8191)
    CAS write new batch_end
```

### `generate_simple()` Limitations

`generate_simple()` uses a single pair of thread-local cells (`TS`, `SEQ`) for the sequence counter. It is designed for **one generator per thread**. Two failure modes to avoid:

1. **Multiple `Generator` instances on the same thread** — they share the same `TS`/`SEQ` cells. The `node_id` field on emitted IDs still differs, so direct duplicates are unlikely, but sequence numbers will interleave unpredictably and the per-instance sequence space is no longer disjoint.
2. **Mixing `generate_simple()` and `generate()` on the same thread** — the atomic path uses its own batch state, independent from `TS`/`SEQ`. Both paths can emit the same `(ts, seq)` pair in the same millisecond and produce duplicate IDs.

For multi-generator or mixed scenarios, use `generate()` exclusively.

### Multi-thread Safety
- `generate()` is fully thread-safe (atomic CAS)
- `generate_simple()` is single-thread only (no atomics, falls back to `generate()` on exhaustion)

### Correctness Proof Sketch
1. Atomic state monotonically increases (timestamp never decreases, sequence never decreases within a timestamp)
2. CAS succeeds only if no other thread modified state
3. Reserved batch range [old_seq, new_batch_end) is disjoint from any other thread's reservation
4. Thread-local cache only consumes from its own reserved range

## 6. Distributed Story

### Node ID Assignment

| Method | Conflict Risk | Use Case |
|--------|--------------|----------|
| `AX_ID_NODE` env var | None (manual) | Docker/k8s with explicit assignment |
| `resolve_node_id()` random | Low (1/1024 per process) | Single-machine multi-process |

**Collision probability:** If N processes start simultaneously without `AX_ID_NODE`, the birthday-bound collision probability is approximately N²/2048 (since there are only 1024 distinct node IDs). For N=32: ~50%. For N=8: ~3%. For N=4: ~0.6%. Above a handful of processes per machine, assign `AX_ID_NODE` explicitly.

### Deployment Recommendations

**Kubernetes:**
```yaml
env:
  - name: AX_ID_NODE
    valueFrom:
      fieldRef:
        fieldPath: metadata.uid   # or pod index
```

**Docker Compose:**
```yaml
services:
  app-1:
    environment:
      - AX_ID_NODE=1
  app-2:
    environment:
      - AX_ID_NODE=2
```

**Single server (no coordination):**
```rust
let gen = Generator::new_auto();   // random node ID, acceptable for < 100 processes
```

### Global Uniqueness Guarantee
IDs are globally unique **iff**:
1. No two nodes share the same `node_id`
2. Clocks are monotonic (or skew ≤ handling threshold)
3. Sequence is not exhausted (handled by spin/yield)

## 7. Comparison with Snowflake/ULID

|  | ax-id | Twitter Snowflake | ULID |
|--|-------|-------------------|------|
| Timestamp bits | 40 | 41 | 48 |
| Node bits | 10 | 10 | 0 |
| Sequence bits | 13 | 12 | — |
| Total throughput | ~8M/ms/node | ~4M/ms/node | — |
| Custom epoch | 2024-01-01 | 2010-11-04 | 1970-01-01 |
| Size | 64-bit | 64-bit | 128-bit |

---

Back to [Usage Guide](usage.md) · [README](../README.md)
