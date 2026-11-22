use serde::Serialize;
use std::path::Path;

use crate::fingerprint::{fingerprint, FingerprintError};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum PathSnapshot {
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "link")]
    Link { target: String },
    #[serde(rename = "directory")]
    Directory { digest: String },
}

pub fn path_snapshot(path: &Path) -> Result<PathSnapshot, FingerprintError> {
  if !lexists(path) {
        return Ok(PathSnapshot::Missing);
    }
    let meta = path.symlink_metadata().map_err(FingerprintError::from)?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(path)?.to_string_lossy().into_owned();
        return Ok(PathSnapshot::Link { target });
    }
    let digest = fingerprint(path)?;
    Ok(PathSnapshot::Directory { digest })
}

fn lexists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}
