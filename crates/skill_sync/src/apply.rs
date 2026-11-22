use chrono::Local;
use skill_sync_core::{fingerprint, path_snapshot};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::plan::{destination_protected, PlanError, SyncReport};

pub fn apply(report: &SyncReport, state: &Path) -> Result<Option<String>, PlanError> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup = state
        .join("backups")
        .join(format!("{stamp}-{}", &Uuid::new_v4().simple().to_string()[..8]));
    let mut journal: Vec<serde_json::Value> = Vec::new();

    let save = |path: &Path, backup: &Path, journal: &mut Vec<serde_json::Value>| -> Result<Option<PathBuf>, PlanError> {
        if !lexists(path) {
            return Ok(None);
        }
        let stripped = path
            .display()
            .to_string()
            .trim_start_matches('/')
            .to_string();
        let target = backup.join(stripped);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(path, &target)?;
        journal.push(serde_json::json!({
            "original": path.display().to_string(),
            "backup": target.display().to_string(),
        }));
        fs::write(
            backup.join("manifest.json"),
            serde_json::to_string_pretty(journal)? + "\n",
        )?;
        Ok(Some(target))
    };

    for action in &report.actions {
        let path = Path::new(&action.path);
        let source = Path::new(&action.source);
        if destination_protected(path, &std::collections::HashSet::new(), action.plugin.is_some())? {
            return Err(PlanError::Msg(format!(
                "Refusing managed destination: {}",
                path.display()
            )));
        }
        if path_snapshot(path)? != action.before {
            return Err(PlanError::Msg(format!(
                "Destination changed during sync: {}",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if action.op == "import" {
            let wanted = action.digest.as_ref().ok_or_else(|| {
                PlanError::Msg("Import action missing digest".into())
            })?;
            if &fingerprint(source)? != wanted {
                return Err(PlanError::Msg(format!(
                    "Source changed during sync: {}",
                    source.display()
                )));
            }
            let temp = path.parent().unwrap().join(format!(".skill-sync-{}", Uuid::new_v4().simple()));
            copy_tree_symlinks(source, &temp)?;
            if &fingerprint(&temp)? != wanted {
                return Err(PlanError::Msg(format!(
                    "Import verification failed; staged copy retained: {}",
                    temp.display()
                )));
            }
            if path_snapshot(path)? != action.before {
                return Err(PlanError::Msg(format!(
                    "Canonical destination changed: {}",
                    path.display()
                )));
            }
            let old = save(path, &backup, &mut journal)?;
            if let Err(err) = fs::rename(&temp, path) {
                if let Some(prev) = old {
                    if !lexists(path) {
                        let _ = fs::rename(prev, path);
                    }
                }
                return Err(err.into());
            }
        } else {
            if !source.join("SKILL.md").is_file() {
                return Err(PlanError::Msg(format!(
                    "Missing canonical source: {}",
                    source.display()
                )));
            }
            if let Some(expected) = &action.digest {
                if path.exists() && &fingerprint(path)? != expected {
                    return Err(PlanError::Msg(format!(
                        "Destination changed during sync: {}",
                        path.display()
                    )));
                }
            }
            if path_snapshot(path)? != action.before {
                return Err(PlanError::Msg(format!(
                    "Destination changed during sync: {}",
                    path.display()
                )));
            }
            let old = save(path, &backup, &mut journal)?;
            #[cfg(unix)]
            let link_err = std::os::unix::fs::symlink(source, path);
            #[cfg(not(unix))]
            let link_err: Result<(), std::io::Error> =
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "symlink"));
            if let Err(err) = link_err {
                if let Some(prev) = old {
                    if !lexists(path) {
                        let _ = fs::rename(prev, path);
                    }
                }
                return Err(err.into());
            }
        }
    }

    Ok(if journal.is_empty() {
        None
    } else {
        Some(backup.display().to_string())
    })
}

fn lexists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

fn copy_tree_symlinks(from: &Path, to: &Path) -> Result<(), PlanError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let src = entry.path();
        let dst = to.join(&name);
        let meta = fs::symlink_metadata(&src)?;
        if meta.file_type().is_symlink() {
            let target = fs::read_link(&src)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                symlink(&target, &dst)?;
            }
            #[cfg(not(unix))]
            {
                let _ = (&target, &dst);
                return Err(PlanError::Msg("Symlink import requires Unix".into()));
            }
        } else if meta.is_dir() {
            copy_tree_symlinks(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = meta.permissions().mode();
                fs::set_permissions(&dst, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}
