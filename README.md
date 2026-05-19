# ax-id

Ultra-fast 64-bit unique ID generator for Rust.

- Sortable IDs
- Distributed-safe
- Zero-allocation
- Compact 64-bit layout
- Thread-safe generation

```rust
use ax_id::Generator;

let gen = Generator::new(1).unwrap();

let id = gen.generate().unwrap();

println!("{}", id);
```

---

## Installation

```toml
[dependencies]
ax-id = "0.1"
```

---

## Quickstart

```rust
use ax_id::Generator;

// Auto node detection
let gen = Generator::new_auto();

// Fast thread-local generation
let id = gen.generate_simple();

// Multi-thread safe generation
let id = gen.generate().unwrap();
```

---

## Why ax-id?

|  | ax-id | UUID v4 | UUID v7 | ULID |
|--|--|--|--|--|
| Size | 64-bit | 128-bit | 128-bit | 128-bit |
| Sortable | ✅ | ❌ | ✅ | ✅ |
| Distributed | ✅ | ✅ | ✅ | ❌ |
| Zero alloc | ✅ | ❌ | ❌ | ❌ |

---

## Features

| Feature | Description |
|---|---|
| `std` | Standard library support |
| `cli` | CLI binary |
| `serde` | Serialization support |
| `sqlx` | SQLx integration |
| `diesel` | Diesel integration |
| `sea-orm` | SeaORM integration |
| `bytemuck` | `Pod` + `Zeroable` |
| `zerocopy` | Zero-copy conversions |
| `rkyv` | Zero-copy archive |
| `borsh` | Binary serialization |
| `arbitrary` | Fuzzing support |

---

## CLI

```bash
# Generate one ID
ax-id

# Generate multiple IDs
ax-id -c 5 -b

# Benchmark
ax-id -c 1000000 --benchmark
```

---

## Notes

- `generate_simple()` is optimized for single-thread local usage
- `generate()` is safe for concurrent multi-generator workloads
- Node IDs support up to 1024 distributed nodes
- IDs are sortable by generation time

---

## Ecosystem

- `doc/usage.md`
- `doc/cli.md`
- `doc/design.md`

---

## License

MIT