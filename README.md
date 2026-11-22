# skill-sync

Native Rust CLI. No
runtime packages or background service. Canonical store: `~/.agents/skills` (the
existing store is retained, including its Git metadata).

## One store, many views

Canonical skills live only in `~/.agents/skills`. Harness-specific folders are
**views**: after `skill-sync`, each discovered home skill root holds symlinks into
the canonical tree, not second copies. skill-sync also scans your home directory
(one or two levels deep) for other `skills` / `skill` folders that already contain
skills and registers them as `auto:<path>` roots.

**For other apps:** read skills from `~/.agents/skills/<slug>/SKILL.md` (and sibling
`references/`, `scripts/`, etc.) instead of maintaining a private cache. If your
app still looks only at its own config directory, users can run `skill-sync` once;
that directory becomes a symlink view. Tools that already point at `~/.agents/skills`
(Kimi, Warp, or a skill root that symlinks there) are treated as native readers and
are not linked twice. Plugin caches, project checkouts, and divergent copies are
unchanged: conflicts stay blocked until `skill-sync --promote HARNESS:SKILL`.

## T3 Code

T3 does not use its own top-level skill folder. For **Antigravity** (the usual default in T3),
the composer `$` skill menu is filled by the server scanning **`~/.gemini/config/skills`** (and
also `~/.gemini/antigravity-cli/skills`, plus project `.agents/skills`). T3’s instance folders
usually symlink `…/config/skills` to that same `~/.gemini/config/skills` tree. skill-sync links
`gemini-config` and the per-instance T3 paths to `~/.agents/skills`.

**Antigravity `$` shows “No skills found” when discovery fails:** T3 reads every `SKILL.md` under
both Gemini skill roots with a fixed **8 MB** scan budget. Maintaining **~600+ skills in each**
root (plus junk like `.git` under `antigravity-cli/skills`) exhausts the budget; discovery aborts
and the UI lists zero skills. Keep **one** populated tree (`~/.gemini/config/skills`) and leave
`~/.gemini/antigravity-cli/skills` empty or minimal, then restart T3 or trigger a provider
refresh. **Codex** / **Claude** threads in T3 still use `~/.codex/skills` and `~/.claude/skills`.

## Build

```sh
cargo build --release
./install.sh    # installs target/release/skill-sync to ~/.local/bin (+ Homebrew bin)
```

Development checkout: [`skill-sync`](skill-sync) launches `target/release/skill-sync`
(build with `cargo build --release`). See [`ARCHITECTURE.md`](ARCHITECTURE.md) and [`PERF.md`](PERF.md).

```sh
skill-sync                     # sync safe skills, report conflicts
skill-sync dry-run             # no writes
skill-sync status              # summary, conflicts, broken links
skill-sync status --json       # every skill and harness link state
skill-sync --help
skill-sync --dry-run --promote claude:some-skill
skill-sync --promote claude:some-skill  # intentionally choose this version
```

Add `~/.agents/skills/my-skill/SKILL.md`, with any `references/`, `scripts/`,
`CHANGELOG.md`, etc., then run `skill-sync`. Edits through a harness link update
the canonical folder. Restart/reload agents that cache skill discovery.

Installed harnesses are detected from config roots or binaries; additional
already-existing skill roots are detected too. Kimi and Warp use the shared
`.agents/skills` store (Kimi discovery: https://moonshotai.github.io/kimi-cli/en/customization/skills.md). Existing higher-priority Kimi skill directories are also detected. This is file synchronization, not verification that each
installed agent/version accepts every skill's frontmatter or harness-specific tools.
Project-only skills, Cursor `skills-cursor`, hidden system skills,
and skills listed in app-managed manifests are deliberately excluded as destinations. Plugin trees are read-only (see below). Existing
project sources and the separate `~/Developer/skills/agent-skills` library are
not imported automatically. Grok Bot's remote skills are not on this Mac.

Whole folders are compared (including references, executable bits, and symlinks;
ignoring `.git`, `.DS_Store`, and `__pycache__` for equivalence). For non-plugin skills, different versions
block that slug everywhere until explicitly promoted. Generic broken links or occupied
non-skill paths also block a slug; these require manual review, not promotion.
Other safe skills still sync. No skill contents are deleted. Replaced directories
and symlinks are renamed into `~/.local/state/skill-sync/backups/<run>/`, with a
`manifest.json` mapping original paths to backups. Old backups are never pruned.
`last-sync.json` records the most recent apply plan. Exit 0 = clean, 2 = conflicts
or broken links remain, 1 = failure. An apply failure may leave earlier safe actions
complete; inspect backups and rerun. Concurrent sync processes are locked out;
close other skill installers while syncing. No background sync is installed.

Tests: `cargo build --release && python3 -m unittest -v` (28 cases, Rust binary by default).
`cargo test` runs fingerprint goldens. `--home /tmp/test-home` isolates runs and disables PATH-based install detection.

## Refreshing pstack after an update

Run `skill-sync` after Cursor updates pstack; no watcher is installed. The tool
selects the newest version directory by directory mtime (name breaks ties) under
`~/.cursor/plugins/cache/cursor-public/pstack/`, using only its immediate
`skills/*/SKILL.md` folders. Numeric marketplace-id copies and nested automation
skills are ignored; no other plugins are automatically imported.

For those slugs, the plugin is authoritative: whole folders refresh the canonical
store, divergent harness skill copies are backed up and linked, and broken links
are repaired. Plugin caches and `skills-cursor` are never written; app-managed
harness destinations are skipped and occupied non-skill real paths still block.
Non-plugin conflicts retain conservative behavior. Use `skill-sync dry-run --json`
to inspect `plugins`, `actions`, and any `skipped_managed_destinations` first.

`./install.sh` installs the native binary to `~/.local/bin/skill-sync` and
`/opt/homebrew/bin/skill-sync` when Homebrew's bin directory exists. No shell PATH edits are needed.
