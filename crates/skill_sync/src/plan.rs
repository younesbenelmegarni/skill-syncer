#[cfg(target_os = "macos")]
use std::os::darwin::fs::MetadataExt;
#[cfg(all(unix, not(target_os = "macos")))]
use std::os::unix::fs::MetadataExt;

use indexmap::IndexMap;
use rayon::prelude::*;
use serde::Serialize;
use skill_sync_core::{fingerprint, path_snapshot, PathSnapshot};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::MANAGED_PARTS;
use crate::discover_home;

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
  #[error("{0}")]
  Msg(String),
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  Fingerprint(#[from] skill_sync_core::fingerprint::FingerprintError),
  #[error(transparent)]
  Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, PlanError>;

fn present(path: &Path) -> bool {
  path.symlink_metadata().is_ok()
}

pub fn managed(root: &Path) -> Result<HashSet<String>> {
  let mut names = HashSet::new();
  if !root.is_dir() {
    return Ok(names);
  }
  for entry in fs::read_dir(root)? {
    let entry = entry?;
    let name = entry.file_name().to_string_lossy().into_owned();
    if !name.starts_with('.') || !name.contains("skills") || !name.ends_with(".json") {
      continue;
    }
    let marker = entry.path();
    let data: serde_json::Value = match fs::read_to_string(&marker) {
      Ok(text) => serde_json::from_str(&text).map_err(|_| {
        PlanError::Msg(format!(
          "Cannot read managed-skill manifest: {}",
          marker.display()
        ))
      })?,
      Err(err) => {
        if err.kind() == std::io::ErrorKind::NotFound {
          continue;
        }
        return Err(PlanError::Msg(format!(
          "Cannot read managed-skill manifest: {}",
          marker.display()
        )));
      }
    };
    if let Some(entries) = data.get("entries").and_then(|e| e.as_object()) {
      for key in entries.keys() {
        names.insert(key.clone());
      }
    }
  }
  Ok(names)
}

fn path_has_managed_part(mut parts: impl Iterator<Item = String>) -> bool {
  parts.any(|p| MANAGED_PARTS.iter().any(|m| *m == p))
}

fn protected(path: &Path, names: &HashSet<String>) -> Result<bool> {
  if names.contains(path.file_name().and_then(|n| n.to_str()).unwrap_or("")) {
    return Ok(true);
  }
  let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
  if fname.starts_with('.') {
    return Ok(true);
  }
  if path.is_symlink() {
    if let Ok(resolved) = path.canonicalize() {
      let parts: HashSet<_> = resolved
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(str::to_string)
        .collect();
      if path_has_managed_part(parts.into_iter()) {
        return Ok(true);
      }
    }
  }
  Ok(false)
}

pub fn destination_protected(path: &Path, names: &HashSet<String>, upstream: bool) -> Result<bool> {
  let parent = path.parent().unwrap_or(path);
  let mut parent_parts: HashSet<String> = parent
    .components()
    .filter_map(|c| c.as_os_str().to_str())
    .map(str::to_string)
    .collect();
  if let Ok(resolved) = parent.canonicalize() {
    parent_parts.extend(
      resolved
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(str::to_string),
    );
  }
  let immutable_parent = path_has_managed_part(parent_parts.into_iter());
  Ok(
    immutable_parent
      || names.contains(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
      || path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
      || (!upstream && protected(path, names)?),
  )
}

fn version_key(path: &Path) -> (i64, i64) {
  #[cfg(unix)]
  {
    let m = path.metadata().ok();
    return (
      m.as_ref().map(|x| x.st_mtime()).unwrap_or(0),
      m.as_ref().map(|x| x.st_mtime_nsec()).unwrap_or(0),
    );
  }
  #[cfg(not(unix))]
  {
    let m = path.metadata().and_then(|x| x.modified().ok());
    let secs = m
      .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
      .map(|d| d.as_secs() as i64)
      .unwrap_or(0);
    (secs, 0)
  }
}

pub fn plugin_sources(home: &Path) -> Result<(HashMap<String, PathBuf>, serde_json::Value)> {
  let root = home
    .join(".cursor")
    .join("plugins")
    .join("cache")
    .join("cursor-public")
    .join("pstack");
  if !root.is_dir() {
    return Ok((HashMap::new(), serde_json::json!({})));
  }
  let mut versions = Vec::new();
  for entry in fs::read_dir(&root)? {
    let entry = entry?;
    let p = entry.path();
    if p.is_dir() && p.join("skills").is_dir() {
      versions.push(p);
    }
  }
  if versions.is_empty() {
    return Ok((HashMap::new(), serde_json::json!({})));
  }
  let current = versions
    .into_iter()
    .max_by(|a, b| {
      version_key(a)
        .cmp(&version_key(b))
        .then_with(|| a.file_name().cmp(&b.file_name()))
    })
    .unwrap();
  let skills_dir = current.join("skills");
  let mut sources = HashMap::new();
  let mut sorted_keys = Vec::new();
  for entry in fs::read_dir(&skills_dir)? {
    let entry = entry?;
    let p = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();
    if name.starts_with('.') {
      continue;
    }
    if p.join("SKILL.md").is_file() {
      sources.insert(name.clone(), p);
      sorted_keys.push(name);
    }
  }
  sorted_keys.sort();
  let plugins = serde_json::json!({
      "pstack": {
          "version": current
              .file_name()
              .map(|n| n.to_string_lossy().into_owned())
              .unwrap_or_default(),
          "path": skills_dir.display().to_string(),
          "skills": sorted_keys,
      }
  });
  Ok((sources, plugins))
}

#[derive(Debug, Clone)]
struct SkillCopy {
  harness: String,
  path: PathBuf,
}

fn inventory(
  roots: &IndexMap<String, PathBuf>,
) -> Result<(
  BTreeMap<String, Vec<SkillCopy>>,
  HashMap<String, HashSet<String>>,
  Vec<String>,
  Vec<String>,
)> {
  let mut skills: BTreeMap<String, Vec<SkillCopy>> = BTreeMap::new();
  let mut blocked = HashMap::new();
  let mut broken = Vec::new();
  let mut excluded = Vec::new();
  for (harness, root) in roots {
    let names = managed(root)?;
    blocked.insert(harness.clone(), names.clone());
    if present(root) && !root.is_dir() {
      return Err(PlanError::Msg(format!(
        "Skills root is not a directory: {}",
        root.display()
      )));
    }
    if !root.is_dir() {
      continue;
    }
    let mut children: Vec<_> = fs::read_dir(root)?
      .filter_map(|e| e.ok())
      .map(|e| e.path())
      .collect();
    children.sort();
    for path in children {
      if path.is_symlink() && !path.exists() {
        broken.push(path.display().to_string());
      }
      if protected(&path, &names)? {
        if path.join("SKILL.md").is_file() {
          excluded.push(path.display().to_string());
        }
        continue;
      }
      if path.join("SKILL.md").is_file() {
        skills
          .entry(path.file_name().unwrap().to_string_lossy().into_owned())
          .or_default()
          .push(SkillCopy {
            harness: harness.clone(),
            path: path.clone(),
          });
      }
    }
  }
  Ok((skills, blocked, broken, excluded))
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncAction {
  pub op: String,
  pub slug: String,
  pub path: String,
  pub source: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub digest: Option<String>,
  pub previous_digest: Option<String>,
  pub before: PathSnapshot,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub plugin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncReport {
  pub plugins: serde_json::Value,
  pub skipped_managed_destinations: Vec<String>,
  pub repairable_broken_links: Vec<String>,
  pub canonical: String,
  pub harnesses: IndexMap<String, String>,
  pub shared_consumers: Vec<String>,
  pub skills: BTreeMap<String, IndexMap<String, String>>,
  pub conflicts: BTreeMap<String, Vec<String>>,
  pub broken_links: Vec<String>,
  pub excluded_managed: Vec<String>,
  pub actions: Vec<SyncAction>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub backup: Option<String>,
  pub applied: bool,
}

fn skill_state(name: &str, root: &Path, slug: &str, dest: &Path) -> Result<String> {
  let target = root.join(slug);
  let dest_skill = dest.join("SKILL.md");
  if name == "agents" && dest_skill.is_file() {
    return Ok("canonical".into());
  }
  if target.is_symlink() {
    if let Ok(resolved) = target.canonicalize() {
      if resolved == dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf()) {
        return Ok("linked".into());
      }
    }
    return Ok("other-link".into());
  }
  if target.join("SKILL.md").is_file() && !target.is_symlink() {
    return Ok("real".into());
  }
  if present(&target) {
    return Ok("occupied".into());
  }
  Ok("missing".into())
}

fn digest_cached(path: &Path, cache: &mut HashMap<PathBuf, String>) -> Result<String> {
  let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
  if let Some(hit) = cache.get(&real) {
    return Ok(hit.clone());
  }
  let value = fingerprint(path)?;
  cache.insert(real, value.clone());
  Ok(value)
}

fn parallel_primes(paths: &[PathBuf]) -> HashMap<PathBuf, String> {
  let unique: HashSet<_> = paths
    .iter()
    .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
    .collect();
  let jobs = std::env::var("SKILL_SYNC_JOBS")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or_else(|| rayon::current_num_threads());
  let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs.max(1))
    .build()
    .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());
  pool.install(|| {
    unique
      .par_iter()
      .filter_map(|p| fingerprint(p).ok().map(|d| (p.clone(), d)))
      .collect()
  })
}

pub fn plan(
  home: &Path,
  roots: &IndexMap<String, PathBuf>,
  promotion: Option<&str>,
) -> Result<SyncReport> {
  let canonical = roots
    .get("agents")
    .cloned()
    .ok_or_else(|| PlanError::Msg("Missing agents root".into()))?;
  let (mut skills, blocked, broken, excluded) = inventory(roots)?;
  let (upstream, plugins) = plugin_sources(home)?;
  for (slug, source) in &upstream {
    skills.entry(slug.clone()).or_default().push(SkillCopy {
      harness: "plugin:pstack".into(),
      path: source.clone(),
    });
  }

  let mut promote_slug: Option<String> = None;
  let mut chosen: Option<PathBuf> = None;
  if let Some(promotion) = promotion {
    let parts: Vec<_> = promotion.splitn(2, ':').collect();
    if parts.len() != 2 {
      return Err(PlanError::Msg("Use --promote HARNESS:SKILL".into()));
    }
    let harness = parts[0];
    let slug = parts[1];
    if !roots.contains_key(harness)
      || slug.is_empty()
      || Path::new(slug).file_name().and_then(|n| n.to_str()) != Some(slug)
    {
      return Err(PlanError::Msg(format!(
        "Unknown harness or invalid skill slug: {promotion}"
      )));
    }
    let pick = roots[harness].join(slug);
    if !skills
      .get(slug)
      .map(|c| c.iter().any(|x| x.path == pick))
      .unwrap_or(false)
    {
      return Err(PlanError::Msg(format!(
        "Promotion source is missing or managed: {}",
        pick.display()
      )));
    }
    promote_slug = Some(slug.into());
    chosen = Some(pick);
  }

  let mut cache = HashMap::new();
  let mut paths_to_hash: Vec<PathBuf> = Vec::new();
  for (_, copies) in &skills {
    for c in copies {
      paths_to_hash.push(c.path.clone());
    }
  }
  for p in upstream.values() {
    paths_to_hash.push(p.clone());
  }
  cache.extend(parallel_primes(&paths_to_hash));

  let mut actions = Vec::new();
  let mut conflicts: BTreeMap<String, Vec<String>> = BTreeMap::new();
  let mut states: BTreeMap<String, IndexMap<String, String>> = BTreeMap::new();
  let mut skipped = Vec::new();

  for (slug, copies) in skills.iter() {
    let dest = canonical.join(slug);
    let plugin = upstream.contains_key(slug);
    if plugin && promote_slug.as_deref() == Some(slug.as_str()) {
      return Err(PlanError::Msg(format!(
        "Plugin owns this skill; remove --promote: {slug}"
      )));
    }
    let source = if plugin {
      upstream[slug].clone()
    } else if promote_slug.as_deref() == Some(slug.as_str()) {
      chosen.clone().unwrap()
    } else if let Some(c) = copies.iter().find(|x| x.harness == "agents") {
      c.path.clone()
    } else {
      copies[0].path.clone()
    };

    let mut variants = HashSet::new();
    for c in copies {
      variants.insert(digest_cached(&c.path, &mut cache)?);
    }
    let mut problems = Vec::new();
    if variants.len() > 1 && promote_slug.as_deref() != Some(slug.as_str()) && !plugin {
      let names: Vec<_> = copies.iter().map(|c| c.harness.as_str()).collect();
      problems.push(format!(
        "different whole-folder contents: {}",
        names.join(", ")
      ));
    }
    for (name, root) in roots {
      let target = root.join(slug);
      if destination_protected(
        &target,
        blocked.get(name).unwrap_or(&HashSet::new()),
        plugin,
      )? {
        if plugin && name != "agents" {
          skipped.push(target.display().to_string());
        } else {
          problems.push(format!("managed destination: {name}"));
        }
      } else if present(&target)
        && !target.join("SKILL.md").is_file()
        && !(plugin && target.is_symlink() && !target.exists())
      {
        problems.push(format!("broken link or occupied non-skill: {name}"));
      }
    }
    let mut per_harness = IndexMap::new();
    for (name, root) in roots {
      per_harness.insert(name.clone(), skill_state(name, root, slug, &dest)?);
    }
    states.insert(slug.clone(), per_harness);
    if !problems.is_empty() {
      conflicts.insert(slug.clone(), problems);
      continue;
    }
    let wanted = digest_cached(&source, &mut cache)?;
    let need_import = !dest.exists()
      || digest_cached(&dest, &mut cache)? != wanted
      || (plugin && dest.is_symlink());
    if need_import {
      actions.push(SyncAction {
        op: "import".into(),
        slug: slug.clone(),
        source: source.display().to_string(),
        path: dest.display().to_string(),
        digest: Some(wanted.clone()),
        previous_digest: if dest.exists() {
          Some(digest_cached(&dest, &mut cache)?)
        } else {
          None
        },
        before: path_snapshot(&dest)?,
        plugin: if plugin { Some("pstack".into()) } else { None },
      });
    }
    let mut visited = HashSet::new();
    visited.insert(
      canonical
        .canonicalize()
        .unwrap_or_else(|_| canonical.clone())
        .join(slug),
    );
    for (name, root) in roots {
      if name != "agents" && discover_home::harness_root_is_canonical_store(root, &canonical) {
        continue;
      }
      let physical = root
        .canonicalize()
        .unwrap_or_else(|_| root.clone())
        .join(slug);
      if visited.contains(&physical) {
        continue;
      }
      visited.insert(physical.clone());
      let target = root.join(slug);
      if plugin
        && destination_protected(&target, blocked.get(name).unwrap_or(&HashSet::new()), true)?
      {
        continue;
      }
      if target.is_symlink() {
        if let (Ok(tgt), Ok(d)) = (target.canonicalize(), dest.canonicalize()) {
          if tgt == d {
            let plugin_ok = if plugin && dest.is_symlink() {
              let link_text = fs::read_link(&target).unwrap_or_default();
              let abs = target
                .parent()
                .map(|p| p.join(&link_text))
                .and_then(|p| p.canonicalize().ok());
              abs == Some(d)
            } else {
              true
            };
            if plugin_ok {
              continue;
            }
          }
        }
      }
      actions.push(SyncAction {
        op: "link".into(),
        slug: slug.clone(),
        path: target.display().to_string(),
        source: dest.display().to_string(),
        digest: if target.exists() {
          Some(digest_cached(&target, &mut cache)?)
        } else {
          None
        },
        previous_digest: None,
        before: path_snapshot(&target)?,
        plugin: if plugin { Some("pstack".into()) } else { None },
      });
    }
  }

  let broken_set: HashSet<_> = broken.iter().cloned().collect();
  let repairs: Vec<String> = actions
    .iter()
    .map(|a| a.path.clone())
    .filter(|p| broken_set.contains(p))
    .collect();
  let broken_links: Vec<String> = broken
    .into_iter()
    .filter(|p| !repairs.contains(p))
    .collect();

  let harnesses: IndexMap<String, String> = roots
    .iter()
    .map(|(k, v)| (k.clone(), v.display().to_string()))
    .collect();

  Ok(SyncReport {
    plugins,
    skipped_managed_destinations: skipped,
    repairable_broken_links: repairs,
    canonical: canonical.display().to_string(),
    harnesses,
    shared_consumers: shared_consumers(home),
    skills: states,
    conflicts,
    broken_links,
    excluded_managed: excluded,
    actions,
    backup: None,
    applied: false,
  })
}

pub fn shared_consumers(home: &Path) -> Vec<String> {
  ["kimi", "warp"]
    .iter()
    .filter(|n| home.join(format!(".{n}")).is_dir())
    .map(|s| (*s).into())
    .collect()
}
