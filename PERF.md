# skill-sync performance report

Historical comparison on macOS 27.2 arm64 (Rust `0.2.0` release vs the retired Python reference). Median of 5 runs, dry-run with `--json`.

| Fixture | Python ref median (retired) | Rust median | Speedup |
|---------|---------------------------|-------------|---------|
| 100 skills × 8 harness roots | 781.2 ms | 616.2 ms | 1.27× |
| 1000 skills × 8 harness roots | 7528.0 ms | 4548.0 ms | 1.66× |

## What improved

- Parallel tree fingerprinting (`rayon`, tunable via `SKILL_SYNC_JOBS`).
- No interpreter startup on the hot path.
- Streaming SHA-256 with 1 MiB buffers.

## What is unchanged

- CLI flags, exit codes, backup layout, conflict rules, and pstack plugin behavior.
- Goldens under `tests/golden/`; regenerate with [`benchmarks/generate_goldens.sh`](benchmarks/generate_goldens.sh).

## Reproduce

```sh
cargo build --release
python3 benchmarks/generate_fixture.py benchmarks/fixtures/skills-1000 --skills 1000
python3 benchmarks/compare.py
```

Raw artifacts: `benchmarks/results/compare-20261109.json`, `benchmarks/results/20261109-python-baseline.json`.

## Remaining bottlenecks

- Full replan still walks every skill slug across every harness root.
- Apply path re-hashes sources for TOCTOU safety (intentional).
- Cross-run digest cache not implemented (would need strict mtime invalidation).
