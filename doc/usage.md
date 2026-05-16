# Usage Guide

Complete examples for every feature and integration.

---

## Table of Contents

- [Basic Generation](#basic-generation)
- [Limitations & Unsupported Scenarios](#limitations--unsupported-scenarios)
- [Feature Flags](#feature-flags)
- [serde](#serde)
- [Database Integrations](#database-integrations)
- [Zero-Copy & Memory](#zero-copy--memory)
- [WASM](#wasm)
- [no_std](#no_std)
- [CLI](#cli)

---

## Basic Generation

```rust
use ax_id::Generator;

// 1. Manual node ID (0–1023)
let gen = Generator::new(42).unwrap();
let id = gen.generate().unwrap();

// 2. Auto-detect node ID
let gen = Generator::new_auto();

// 3. Fast single-thread path (no atomics).
//    Only use ONE generator per thread with this method.
let id = gen.generate_simple();

// 4. Safe path — atomic, multi-thread and multi-generator safe
let id = gen.generate().unwrap();              // 08b53edc41582000
println!("hex:  {}", id);              // 08b53edc41582000
println!("raw:  {}", u64::from(id));     // 627480090874970112
println!("ts:   {}", id.timestamp_ms());
println!("node: {}", id.node_id());
println!("seq:  {}", id.raw_sequence());

// 5. Parse from string
let from_hex: ax_id::Id = "08b53edc41582000".parse().unwrap();
let from_dec: ax_id::Id = "627480090874970112".parse().unwrap();
```

**Note:** `generate_simple()` falls back to `generate()` automatically if the per-millisecond sequence is exhausted (8192 IDs/ms). You never need to handle this manually.

---

## Limitations & Unsupported Scenarios

| Scenario | Status | Why | What to use instead |
|----------|--------|-----|-------------------|
| Multiple `Generator` instances, same thread, `generate_simple()` | ❌ **Not supported** | Single thread-local counter shared across all generators | `generate()` |
| Multiple `Generator` instances, same thread, `generate()` | ✅ **Supported** | `instance_tag` prevents stale batch reuse | `generate()` |
| Clock skew > 1 second backward | ❌ **Errors** | Would break monotonicity guarantee | Fix system time, then retry |
| Timestamp wrap after ~2058 | ⚠️ **Silent wrap** | 40-bit field overflows | Migrate format before 2058 |
| `no_std` ID generation | ❌ **Not supported** | Requires `SystemTime` + atomics | Use `std` feature |
| Sequence > 8192 in same ms on same node | ⚠️ **Auto-handled** | Spin-loop until next ms | No action needed |

### `generate_simple()` — One Generator Per Thread

```rust
// ✅ OK: one generator, one thread
let gen = Generator::new_auto();
for _ in 0..1_000_000 {
    let id = gen.generate_simple();
}

// ❌ WRONG: interleaving generators on same thread
let gen_a = Generator::new(1).unwrap();
let gen_b = Generator::new(2).unwrap();
for _ in 0..100_000 {
    let id_a = gen_a.generate_simple();  // overwrites shared counter
    let id_b = gen_b.generate_simple();  // may produce duplicate
}

// ✅ CORRECT: use atomic generate() for multi-generator-per-thread
for _ in 0..100_000 {
    let id_a = gen_a.generate().unwrap();
    let id_b = gen_b.generate().unwrap();
}
```

---

## Feature Flags

```toml
[dependencies]
# Default: std + CLI binary
ax-id = "0.1"

# Embedded / WASM: no_std, no alloc beyond core
ax-id = { version = "0.1", default-features = false }

# With integrations
ax-id = { version = "0.1", features = ["serde", "sqlx", "bytemuck"] }
```

| Feature | When to use |
|---------|-------------|
| `std` | You need `Generator` or the CLI. |
| `serde` | Serializing IDs in JSON, MessagePack, etc. |
| `sqlx` | Storing IDs in PostgreSQL, SQLite, or MySQL. |
| `diesel` | Using Diesel ORM with `BIGINT` columns. |
| `sea-orm` | Using SeaORM. |
| `bytemuck` | Casting `Id` to/from raw bytes safely. |
| `zerocopy` | Zero-copy deserialization from network buffers. |
| `rkyv` | Archive support. |
| `borsh` | Compact binary format. |

---

## serde

### Default: serialize as `u64`

```rust
use ax_id::Id;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Event {
    id: Id,
}

let event = Event { id: Id(42) };
let json = serde_json::to_string(&event).unwrap();
assert_eq!(json, r#"{"id":42}"#);
```

### Hex string format

```rust
use ax_id::Id;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Event {
    #[serde(with = "ax_id::serde::hex")]
    id: Id,
}

let event = Event { id: Id(42) };
let json = serde_json::to_string(&event).unwrap();
assert_eq!(json, r#"{"id":"000000000000002a"}"#);
```

---

## Database Integrations

### sqlx

```rust
use ax_id::Id;
use sqlx::types::Type;

// Id maps to BIGINT in all supported databases.
// Just use it in queries:

// PostgreSQL
sqlx::query("INSERT INTO events (id) VALUES ($1)")
    .bind(Id::from(42))
    .execute(&pool)
    .await?;

// SQLite
sqlx::query("INSERT INTO events (id) VALUES (?1)")
    .bind(Id::from(42))
    .execute(&pool)
    .await?;
```

### diesel

```rust
use ax_id::Id;
use diesel::{prelude::*, sql_types::BigInt};

table! {
    events (id) {
        id -> BigInt,
    }
}

#[derive(Insertable, Queryable)]
#[diesel(table_name = events)]
struct Event {
    id: Id,
}
```

### sea-orm

```rust
use ax_id::Id;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "events")]
struct Event {
    #[sea_orm(primary_key)]
    id: Id,
}
```

---

## Zero-Copy & Memory

### bytemuck

```rust
use ax_id::Id;
use bytemuck::{Pod, Zeroable};

let id = Id(0xdeadbeef);
let bytes: [u8; 8] = bytemuck::bytes_of(&id).try_into().unwrap();
let back: &Id = bytemuck::from_bytes(&bytes);
assert_eq!(id, *back);
```

### zerocopy

```rust
use ax_id::Id;
use zerocopy::{IntoBytes, FromBytes};

let id = Id(42);
let bytes = id.as_bytes();
let back = Id::read_from_bytes(bytes).unwrap();
assert_eq!(id, back);
```

---

## WASM

`ax-id` works on `wasm32-unknown-unknown` without any extra configuration. The `ax-rnd` dependency is pure Rust and does not require platform-specific backends.

```toml
[dependencies]
ax-id = { version = "0.1", default-features = false }
```

```bash
cargo build --target wasm32-unknown-unknown
```

---

## no_std

```toml
[dependencies]
ax-id = { version = "0.1", default-features = false }
```

In `no_std` you can still use `Id` for parsing, formatting, and bit extraction. Generation requires `std` because it needs `SystemTime` and atomics.

```rust
#![no_std]
extern crate alloc;

use ax_id::Id;

let id = Id(0x08b53edc41582000);
assert_eq!(id.to_string(), "08b53edc41582000");
```

---

## CLI

Full command reference: [CLI Reference](cli.md)

Quick examples:

```bash
ax-id                 # one ID
ax-id -c 10 -b       # 10 IDs, space-separated
ax-id -c 1000000 --benchmark
```

---

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `AX_ID_NODE` | Override node ID (0–1023). Checked before random fallback. |

---

## Performance Tips

1. **Single thread, one generator:** Use `generate_simple()`. It skips atomics entirely. **Do not** call `generate_simple()` from multiple `Generator` instances on the same thread.
2. **Multi thread OR multiple generators per thread:** Use `generate()`. Lock-free atomic batching (~512 IDs per CAS). Safe with any number of threads and generator instances.
3. **Node 0:** Avoid if possible. Use `AX_ID_NODE` or `new_auto()` for entropy.
4. **Long-running processes:** Clock skew ≤1s is auto-recovered. Skew >1s returns `ClockSkew` error — fix system time and retry.

---

Back to [README](../README.md)
