# CLI Reference

## Install

```bash
cargo install ax-id
# or from source:
cargo install --path .
```

Requires `cli` feature (enabled by default).

## Commands

### Generate IDs

```bash
# Default: one ID in hex
ax-id
# → 08b53edc41582000

# Count
ax-id -c 5
# → 08b53edc41582000
# → 08b53edc41582001
# → ...

# Bulk (space-separated single line)
ax-id -c 5 -b
# → 08b53edc41582000 08b53edc41582001 ...

# Raw format (decimal)
ax-id -c 3 --format raw
# → 627480090874970112
# → 627480090874970113
# → ...

# Timestamp mode (fixed custom epoch)
ax-id -c 3 --mode timestamp
```

### Modes

| Mode | Description | Use case |
|------|-------------|----------|
| `simple` *(default)* | `generate_simple()` — thread-local, fastest | Single-thread bulk generation |
| `monotonic` | `generate()` — atomic, monotonic | Multi-thread safety |
| `distributed` | `generate()` with shared generator | Same as monotonic |
| `timestamp` | Fixed timestamp (1000ms epoch) | Testing / deterministic IDs |

```bash
ax-id --mode simple      # default
ax-id --mode monotonic
ax-id --mode timestamp
```

### Benchmark

```bash
# Benchmark 1M IDs
ax-id -c 1000000 --benchmark

# Benchmark with specific mode
ax-id -c 1000000 --benchmark --mode simple
```

### Environment Variables

| Variable | Effect |
|----------|--------|
| `AX_ID_NODE` | Override node ID (0–1023). Example: `AX_ID_NODE=7 ax-id` |

## Options Reference

| Short | Long | Default | Description |
|-------|------|---------|-------------|
| `-c` | `--count` | `1` | Number of IDs to generate |
| `-f` | `--format` | `hex` | Output format: `hex` or `raw` |
| `-m` | `--mode` | `simple` | Generation mode |
| `-b` | `--bulk` | — | Output space-separated single line |
| — | `--benchmark` | — | Run benchmark instead of output |

---

Back to [Usage Guide](usage.md)
