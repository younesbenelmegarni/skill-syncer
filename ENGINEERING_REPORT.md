# SkillSync Rust refactor — engineering report

## 1. Architecture changes

**Before:** One stdlib Python script owned detect, inventory, plan, apply, and CLI.

**After:** Workspace with `skill_sync_core` (fingerprints + snapshots) and `skill_sync` (CLI + plan/apply). [`skill-sync`](skill-sync) runs the release binary only.

## 2. Non-Rust tooling (dev only)

| Artifact | Why |
|--------|-----|
| [`test_skill_sync.py`](test_skill_sync.py) | Integration suite (invokes Rust CLI) |
| [`benchmarks/generate_fixture.py`](benchmarks/generate_fixture.py) | Synthetic homes for scale tests |
| [`benchmarks/compare.py`](benchmarks/compare.py) | Timing harness when hyperfine is absent |

## 3. Rust modules introduced

| Module | Responsibility |
|--------|----------------|
| `skill_sync_core::fingerprint` | Stable tree digest (`repr` + SHA-256) |
| `skill_sync_core::snapshot` | `before` state for TOCTOU |
| `skill_sync::detect` | Harness roots + shared consumers |
| `skill_sync::plan` | Inventory, pstack sources, actions |
| `skill_sync::apply` | Backup journal, import/link |
| `skill_sync` binary | clap CLI, lock, JSON/text output |

## 4–5. Performance and resources

See [`PERF.md`](PERF.md). Historical dry-run comparison vs the old Python implementation: **1.27×** at 100 skills, **1.66×** at 1000 skills on this Mac.

## 6. Testing

- `python3 -m unittest -v` — integration tests against `./target/release/skill-sync`.
- `cargo test` — apply race + fingerprint goldens (`tests/golden/`).

## 7. Compatibility

CLI commands, flags, exit codes, backup paths, and JSON field names are preserved. Install path is still global `skill-sync` on PATH via `install.sh`.

## 8. Remaining bottlenecks

Plan complexity is still O(skills × harnesses × tree size). Apply re-fingerprints for safety. No persistent digest cache.

## 9. Recommended next steps

1. Optional cross-run digest cache with `(path, mtime_ns, size)` keys.
2. Reduce duplicate hashing during apply when `before` snapshot is unchanged.
3. `hyperfine` in CI when available; 10k-skill fixture profile.
4. `cargo install --path crates/skill_sync` publish flow / GitHub release artifacts.
