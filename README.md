# ax-id

Ultra-fast 64-bit unique ID generator for Rust. Sortable, distributed-safe, and zero-allocation.

```rust
use ax_id::Generator;

let gen = Generator::new(1).unwrap();
let id = gen.generate().unwrap();

println!("{}", id); // 08b53edc41582000
```

## Quickstart

```toml
[dependencies]
ax-id = "0.1"
```

```rust
use ax_id::Generator;

// Auto-detect node ID from environment or random entropy
let gen = Generator::new_auto();

// Fast path — thread-local batching, no atomics.
// Constraints: ONE generator per thread, and do not mix with
// generate() on the same thread (the two paths track sequence
// independently and would overlap in the same ms).
let id = gen.generate_simple();

// Safe path — atomic coordination across threads, multi-generator safe.
let id = gen.generate().unwrap();
```

## Why ax-id?

|  | ax-id | UUID v4 | UUID v7 | ULID |
|--|-------|---------|---------|------|
| Size | 64-bit | 128-bit | 128-bit | 128-bit |
| Sortable | ✅ | ❌ | ✅ | ✅ |
| Distributed | ✅ up to 1024 nodes | ✅ | ✅ | ❌ |
| Throughput | ~12M IDs/s | ~3M | ~5M | ~4M |
| Zero alloc | ✅ | ❌ | ❌ | ❌ |

*Benchmarked on Apple M2, single thread. See [`benches/`](benches/) for details.*

## Layout

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0|                         timestamp (40)                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         node id (10)          |        sequence (13)          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **40-bit timestamp**: milliseconds since 2024-01-01 (~34 years range)
- **10-bit node ID**: 0–1023, auto-resolved from `AX_ID_NODE` env or random entropy
- **13-bit sequence**: 8192 IDs per millisecond per node

## Features

| Feature | Description |
|---------|-------------|
| `std` *(default)* | `Generator`, `SystemTime` parsing, CLI binary |
| `cli` *(default)* | `ax-id` command-line tool |
| `serde` | `Serialize` / `Deserialize` as `u64` or hex string |
| `sqlx` | `Type`, `Encode`, `Decode` for PostgreSQL / SQLite / MySQL |
| `diesel` | `ToSql` / `FromSql` for `BIGINT` |
| `sea-orm` | `TryFromU64`, `TryGetable`, `FromQueryResult` |
| `bytemuck` | `Pod` + `Zeroable` |
| `zerocopy` | `IntoBytes` + `FromBytes` |
| `rkyv` | Zero-copy archive |
| `borsh` | Compact binary serialization |
| `arbitrary` | `proptest` / `cargo-fuzz` support |

## CLI

```bash
# Generate one ID
$ ax-id
08b53edc41582000

# Bulk generate
$ ax-id -c 5 -b
08b53edc41582000 08b53edc41582001 08b53edc41582002 08b53edc41582003 08b53edc41582004

# Benchmark
$ ax-id -c 1000000 --benchmark
```

## Ecosystem

- [Usage examples & integration guides](doc/usage.md)
- [CLI command reference](doc/cli.md)
- [Design & correctness specification](doc/design.md)

## License

MIT
