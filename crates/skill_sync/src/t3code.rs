//! T3 Code stores per-provider skill roots under `~/.t3/userdata/providers/`.
//! Antigravity (the common default in T3) uses `…/<instance>/config/skills` for the
//! composer `$` picker and `…/<instance>/antigravity-cli/skills` for the CLI bundle.

use indexmap::IndexMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_roots(home: &Path) -> IndexMap<String, PathBuf> {
  let mut roots = IndexMap::new();
  let providers = home.join(".t3/userdata/providers");
  let antigravity = providers.join("antigravity");
  if !antigravity.is_dir() {
    return roots;
  }
  let mut instances: Vec<_> = fs::read_dir(&antigravity)
    .into_iter()
    .flatten()
    .filter_map(|e| e.ok())
    .filter(|e| e.path().is_dir())
    .collect();
  instances.sort_by_key(|e| e.file_name());
  for entry in instances {
    let id = entry.file_name().to_string_lossy().into_owned();
    let short = if id.len() > 8 { &id[..8] } else { id.as_str() };
    let base = entry.path();
    let config_skills = base.join("config/skills");
    if config_skills.is_dir() {
      roots.insert(format!("t3code-antigravity-{short}-config"), config_skills);
    }
    let cli_skills = base.join("antigravity-cli/skills");
    if cli_skills.is_dir() {
      roots.insert(format!("t3code-antigravity-{short}-cli"), cli_skills);
    }
  }
  roots
}
