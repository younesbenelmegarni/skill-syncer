#!/usr/bin/env python3
"""Build synthetic --home trees for skill-sync benchmarks."""
import argparse
import os
import shutil
import stat
import sys
from pathlib import Path

HARNESS_DIRS = [
    ".agents/skills",
    ".cursor/skills",
    ".claude/skills",
    ".codex/skills",
    ".gemini/skills",
    ".config/opencode/skills",
    ".factory/skills",
    ".hermes/skills",
    ".grok/skills",
]


def write_skill(root: Path, slug: str, body: str = "skill body", large_ref: bool = False) -> None:
    skill = root / slug
    (skill / "references").mkdir(parents=True, exist_ok=True)
    (skill / "scripts").mkdir(exist_ok=True)
    (skill / "SKILL.md").write_text(f"---\nname: {slug}\n---\n{body}\n")
    (skill / "references/rules.md").write_text("rules for " + slug + ("\n" + ("x" * 65536) if large_ref else ""))
    (skill / "CHANGELOG.md").write_text("changelog\n")
    run = skill / "scripts/run"
    run.write_text("#!/bin/sh\nexit 0\n")
    run.chmod(run.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def ensure_harness_configs(home: Path) -> None:
    for rel in (".cursor", ".claude", ".codex", ".gemini", ".config/opencode",
                ".factory", ".hermes", ".grok"):
        (home / rel).mkdir(parents=True, exist_ok=True)


def generate(home: Path, count: int, nested: int = 0, large: bool = False) -> None:
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True)
    ensure_harness_configs(home)
    for harness in HARNESS_DIRS:
        (home / harness).mkdir(parents=True, exist_ok=True)
    width = max(5, len(str(count - 1)))
    for i in range(count):
        slug = f"skill-{i:0{width}d}"
        for harness in HARNESS_DIRS:
            write_skill(home / harness, slug, f"body-{i}", large_ref=large and i % 100 == 0)
        if nested and i == 0:
            deep = home / HARNESS_DIRS[0] / slug / "references" / "deep"
            deep.mkdir(parents=True)
            leaf = deep
            for d in range(nested):
                leaf = leaf / f"level-{d}"
                leaf.mkdir()
            (leaf / "leaf.md").write_text("deep\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("home", type=Path)
    parser.add_argument("--skills", type=int, default=100)
    parser.add_argument("--nested-depth", type=int, default=0)
    parser.add_argument("--large-refs", action="store_true")
    args = parser.parse_args()
    generate(args.home.expanduser().resolve(), args.skills, args.nested_depth, args.large_refs)
    print(f"Wrote {args.skills} skills under {args.home}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
