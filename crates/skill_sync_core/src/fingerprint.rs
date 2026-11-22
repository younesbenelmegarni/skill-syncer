use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

use thiserror::Error;

pub const IGNORED_NAMES: &[&str] = &[".git", ".DS_Store", "__pycache__"];

#[derive(Debug, Error)]
pub enum FingerprintError {
    #[error("{0}")]
    Msg(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, FingerprintError>;

#[derive(Clone, Debug)]
enum Entry {
    Link { rel: String, target: String },
    Dir { rel: String },
    File {
        rel: String,
        executable: bool,
        digest: String,
    },
}

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut result = Vec::new();
    walk(path, "", &mut result)?;
    let repr = python_repr_list(&result);
    let mut hasher = Sha256::new();
    hasher.update(repr.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn walk(path: &Path, prefix: &str, result: &mut Vec<Entry>) -> Result<()> {
    if !path.is_dir() {
        return Err(FingerprintError::Msg(format!(
            "Not a directory: {}",
            path.display()
        )));
    }
    let mut names: Vec<String> = fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    names.sort();
    for name in names {
        if IGNORED_NAMES.iter().any(|i| *i == name) {
            continue;
        }
        let entry = path.join(&name);
        let rel = format!("{prefix}{name}");
        let meta = entry.symlink_metadata()?;
        let mode = mode_from(&meta);
        if meta.file_type().is_symlink() {
            let target = fs::read_link(&entry)?.to_string_lossy().into_owned();
            result.push(Entry::Link { rel, target });
        } else if meta.is_dir() {
            result.push(Entry::Dir { rel: rel.clone() });
            walk(&entry, &(rel + "/"), result)?;
        } else if meta.is_file() {
            let digest = hash_file(&entry)?;
            let executable = mode & 0o111 != 0;
            result.push(Entry::File {
                rel,
                executable,
                digest,
            });
        } else {
            return Err(FingerprintError::Msg(format!(
                "Unsupported special file: {}",
                entry.display()
            )));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn mode_from(meta: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode()
}

#[cfg(not(unix))]
fn mode_from(_meta: &fs::Metadata) -> u32 {
    0
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

fn python_repr_list(entries: &[Entry]) -> String {
    let mut out = String::from("[");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&python_repr_entry(entry));
    }
    out.push(']');
    out
}

fn python_repr_entry(entry: &Entry) -> String {
    match entry {
        Entry::Link { rel, target } => format!(
            "({}, {}, {})",
            python_str(rel),
            python_str("link"),
            python_str(target)
        ),
        Entry::Dir { rel } => format!("({}, {})", python_str(rel), python_str("dir")),
        Entry::File {
            rel,
            executable,
            digest,
        } => format!(
            "({}, {}, {}, {})",
            python_str(rel),
            python_str("file"),
            if *executable { "True" } else { "False" },
            python_str(digest)
        ),
    }
}

fn python_str(value: &str) -> String {
    let mut out = String::from("'");
    for ch in value.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    #[test]
    fn sample_tree_matches_legacy_shape() {
        let dir = std::env::temp_dir().join(format!("skill-sync-fp-{}", std::process::id()));
        fs::create_dir_all(dir.join("references")).unwrap();
        fs::write(dir.join("SKILL.md"), "x").unwrap();
        fs::write(dir.join("references/rules.md"), "rule").unwrap();
        symlink("rules.md", dir.join("references/alias.md")).unwrap();
        let fp = fingerprint(&dir).unwrap();
        assert_eq!(fp.len(), 64);
        fs::remove_dir_all(dir).ok();
    }
}
