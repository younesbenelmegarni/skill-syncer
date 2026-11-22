# Architecture

## Overview

skill-sync is a Rust workspace CLI. The repo [`skill-sync`](skill-sync) wrapper runs
`target/release/skill-sync` after `cargo build --release`.

## Crates

| Crate | Responsibility |
|-------|----------------|
| [`skill_sync_core`](crates/skill_sync_core) | Tree fingerprinting (`repr` + SHA-256), `path_snapshot` |
| [`skill_sync`](crates/skill_sync) | CLI, harness detection, plan/apply, locking, JSON report |

## Data flow

```
detect(home) → inventory + plugin_sources → plan → apply (optional) → stdout/last-sync.json
```

`plan` primes a parallel digest cache before building import/link actions. `apply` enforces `before` snapshots and never deletes skill contents (backup rename journal).

## Tests and goldens

- [`test_skill_sync.py`](test_skill_sync.py) — integration tests against the release binary.
- `cargo test` — unit/integration tests (e.g. apply race) and fingerprint goldens in `tests/golden/`.
- Regenerate goldens: [`benchmarks/generate_goldens.sh`](benchmarks/generate_goldens.sh) (`cargo run --example generate_goldens`).

## Build and install

```sh
cargo build --release
./install.sh   # copies to ~/.local/bin and /opt/homebrew/bin when present
```

Or invoke `./target/release/skill-sync` directly.

## Benchmarks

See [`benchmarks/README.md`](benchmarks/README.md) and [`PERF.md`](PERF.md).
