#!/usr/bin/env python3
"""Simple timing harness when hyperfine is unavailable (Rust CLI only)."""
import json
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CASES = [
    ('skills-100', ROOT / 'benchmarks/fixtures/skills-100'),
    ('skills-1000', ROOT / 'benchmarks/fixtures/skills-1000'),
]
RUNS = 5
BIN = ROOT / 'target/release/skill-sync'


def median_ms(cmd):
    samples = []
    for _ in range(RUNS):
        start = time.perf_counter()
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        samples.append((time.perf_counter() - start) * 1000)
    return statistics.median(samples)


def main():
    if not BIN.is_file():
        raise SystemExit(f'missing {BIN}; run cargo build --release')
    out = []
    for name, home in CASES:
        cmd = [str(BIN), '--home', str(home), 'dry-run', '--json']
        out.append({'fixture': name, 'command': 'rust-dry-run', 'median_ms': median_ms(cmd), 'runs': RUNS})
    path = ROOT / 'benchmarks/results/compare-latest.json'
    path.write_text(json.dumps(out, indent=2) + '\n')
    print(path)
    for row in out:
        print(f"{row['fixture']:12} {row['command']:16} {row['median_ms']:.1f} ms")


if __name__ == '__main__':
    main()
