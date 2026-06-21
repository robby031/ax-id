# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-06-21

### Changed

- Bump `clap` 4.5 → 4.6.1
- Bump `zerocopy` 0.8.49 → 0.8.52
- Bump `diesel` 2.3.9 → 2.3.10
- Bump `borsh` 1.6.1 → 1.7.0
- Bump dev: `uuid` 1.12 → 1.23.3
- Bump dev: `ulid` 1.1 → 1.2.1
- Bump dev: `sonyflake` 0.4.0 → 0.5.1
- Bump dev: `serde_json` 1.0.149 → 1.0.150

## [0.1.2] - 2026-05-20

### Changed

- Bump `ax-rnd` 0.1.3 → 0.1.4

## [0.1.1] - 2026-05-19

### Changed

- Pin all dependency versions to specific patch releases
- Bump `ax-rnd` 0.1 → 0.1.3
- Bump `sqlx` 0.8 → 0.8.6
- Bump `diesel` 2.2 → 2.3.9
- Bump `zerocopy` 0.8 → 0.8.48
- Bump `sea-orm` 1.1 → 1.1.20
- Bump `rkyv` 0.8 → 0.8.16
- Bump `borsh` 1.5 → 1.6.1
- Bump `serde` 1.0 → 1.0.228
- Bump `arbitrary` 1.4 → 1.4.2
- Bump `bytemuck` 1.14 → 1.25.0

### Added

- Add `authors` field in `Cargo.toml`

## [0.1.0] - 2026-05-16

### Added

- Initial release: ultra-fast 64-bit unique ID generator and parser
- Core `no_std`-compatible implementation
- Optional features: `std`, `cli`, `serde`, `arbitrary`, `bytemuck`, `zerocopy`, `sqlx`, `diesel`, `sea-orm`, `rkyv`, `borsh`
- CLI binary via `clap` (behind `cli` feature)
- Benchmark suite comparing against `uuid`, `ulid`, `snowflake`, `ksuid`, `fastid`

[Unreleased]: https://github.com/robby031/ax-id/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/robby031/ax-id/compare/v0.1.2...v0.2.2
[0.1.2]: https://github.com/robby031/ax-id/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/robby031/ax-id/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/robby031/ax-id/releases/tag/v0.1.0
