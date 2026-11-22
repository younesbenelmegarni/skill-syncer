use indexmap::IndexMap;
use std::path::{Path, PathBuf};

use crate::constants::{ALIASES, EXISTING, HARNESSES};
use crate::discover_home;
use crate::t3code;

pub fn detect(home: &Path, binaries: bool) -> (IndexMap<String, PathBuf>, Vec<String>) {
  let mut roots = IndexMap::new();
  let canonical = home.join(".agents").join("skills");
  roots.insert("agents".into(), canonical.clone());
  for h in HARNESSES {
    let config = home.join(h.config);
    let on_path = binaries && which::which(h.binary).is_ok();
    if config.is_dir() || on_path {
      roots.insert(h.name.into(), home.join(h.skills));
    }
  }
  for name in EXISTING {
    let path = home.join(format!(".{name}")).join("skills");
    if path.is_dir() {
      roots.insert((*name).into(), path);
    }
  }
  for (name, rel) in ALIASES {
    let path = home.join(rel);
    if path.is_dir() {
      roots.insert((*name).into(), path);
    }
  }
  for (name, path) in t3code::discover_roots(home) {
    roots.insert(name, path);
  }
  for (name, path) in discover_home::discover_home_skill_roots(home, &canonical, &roots) {
    roots.insert(name, path);
  }
  let mut shared = Vec::new();
  for name in ["kimi", "warp"] {
    let dir = home.join(format!(".{name}"));
    let on_path = binaries && which::which(name).is_ok();
    if dir.is_dir() || on_path {
      shared.push(name.into());
    }
  }
  (roots, shared)
}
