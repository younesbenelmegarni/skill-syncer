# skill-sync benchmarks

## Fixtures

```sh
python3 benchmarks/generate_fixture.py benchmarks/fixtures/skills-100 --skills 100
python3 benchmarks/generate_fixture.py benchmarks/fixtures/skills-1000 --skills 1000
```

Set `SKIP_10K=1` to skip the 10k case in the baseline script.

## Baseline (Rust CLI)

```sh
chmod +x benchmarks/run_baseline.sh
./benchmarks/run_baseline.sh
```

Results land in `benchmarks/results/<date>-rust-baseline.json`.

## Quick timing

After `cargo build --release`:

```sh
python3 benchmarks/compare.py
# or hyperfine --warmup 1 './target/release/skill-sync --home … dry-run --json'
```

## Goldens

```sh
./benchmarks/generate_goldens.sh
```

## Environment

Record `platform.platform()`, CPU count, and `SKILL_SYNC_JOBS` when comparing parallel digest throughput.
