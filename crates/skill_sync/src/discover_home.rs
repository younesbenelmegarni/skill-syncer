//! Scan the home directory for skill roots not listed in the harness tables.

use indexmap::IndexMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::MANAGED_PARTS;

const SKILL_DIR_NAMES: &[&str] = &["skills", "skill"];

pub fn discover_home_skill_roots(
  home: &Path,
  canonical: &Path,
  already: &IndexMap<String, PathBuf>,
) -> IndexMap<String, PathBuf> {
  let mut known = Vec::new();
  for path in already.values() {
    push_resolved(path, &mut known);
  }
  push_resolved(canonical, &mut known);

  let mut found = IndexMap::new();
  let mut seen_paths = Vec::new();
  for candidate in skill_root_candidates(home) {
    if should_skip(&candidate, home, canonical, &known, &seen_paths) {
      continue;
    }
    if !has_skill_children(&candidate) {
      continue;
    }
    if root_points_at_canonical(&candidate, canonical) {
      continue;
    }
    push_resolved(&candidate, &mut seen_paths);
    let rel = candidate
      .strip_prefix(home)
      .unwrap_or(&candidate)
      .to_string_lossy()
      .replace('\\', "/");
    let key = format!("auto:{rel}");
    if found.contains_key(&key) {
      continue;
    }
    found.insert(key, candidate);
  }
  found
}

fn skill_root_candidates(home: &Path) -> Vec<PathBuf> {
  let mut out = Vec::new();
  for name in SKILL_DIR_NAMES {
    let direct = home.join(name);
    if direct.is_dir() {
      out.push(direct);
    }
  }
  let Ok(home_entries) = fs::read_dir(home) else {
    return out;
  };
  for entry1 in home_entries.flatten() {
    let p1 = entry1.path();
    if !p1.is_dir() {
      continue;
    }
    for name in SKILL_DIR_NAMES {
      let p = p1.join(name);
      if p.is_dir() {
        out.push(p);
      }
    }
    let Ok(depth2) = fs::read_dir(&p1) else {
      continue;
    };
    for entry2 in depth2.flatten() {
      let p2 = entry2.path();
      if !p2.is_dir() {
        continue;
      }
      for name in SKILL_DIR_NAMES {
        let p = p2.join(name);
        if p.is_dir() {
          out.push(p);
        }
      }
    }
  }
  out
}

fn has_skill_children(path: &Path) -> bool {
  let Ok(entries) = fs::read_dir(path) else {
    return false;
  };
  entries.flatten().any(|e| {
    let p = e.path();
    p.is_dir() && p.join("SKILL.md").is_file()
  })
}

fn path_has_managed_part(path: &Path) -> bool {
  path
    .components()
    .filter_map(|c| c.as_os_str().to_str())
    .any(|p| MANAGED_PARTS.iter().any(|m| *m == p))
}

/// Harness skill root is already the canonical store (e.g. symlink to `~/.agents/skills`).
pub fn harness_root_is_canonical_store(root: &Path, canonical: &Path) -> bool {
  root_points_at_canonical(root, canonical)
}

fn under_plugin_cache(home: &Path, path: &Path) -> bool {
  let cache = home.join(".cursor/plugins/cache");
  path.starts_with(&cache) || path.starts_with(&cache.canonicalize().unwrap_or(cache))
}

fn should_skip(
  path: &Path,
  home: &Path,
  canonical: &Path,
  known: &[PathBuf],
  seen: &[PathBuf],
) -> bool {
  if path == canonical {
    return true;
  }
  if path.starts_with(canonical) {
    return true;
  }
  if path_has_managed_part(path) {
    return true;
  }
  if under_plugin_cache(home, path) {
    return true;
  }
  if same_resolved_path(path, known) || same_resolved_path(path, seen) {
    return true;
  }
  false
}

fn root_points_at_canonical(root: &Path, canonical: &Path) -> bool {
  if root.is_symlink() {
    let Ok(target) = fs::read_link(root) else {
      return false;
    };
    let dest = if target.is_absolute() {
      target
    } else {
      root.parent().map(|p| p.join(&target)).unwrap_or(target)
    };
    return same_as(&dest, canonical);
  }
  same_as(root, canonical)
}

fn push_resolved(path: &Path, vec: &mut Vec<PathBuf>) {
  if let Ok(r) = path.canonicalize() {
    vec.push(r);
  } else {
    vec.push(path.to_path_buf());
  }
}

fn same_resolved_path(path: &Path, known: &[PathBuf]) -> bool {
  known.iter().any(|k| same_as(path, k))
}

fn same_as(a: &Path, b: &Path) -> bool {
  match (a.canonicalize(), b.canonicalize()) {
    (Ok(x), Ok(y)) => x == y,
    _ => a == b,
  }
}
