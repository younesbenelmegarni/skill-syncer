//! Regenerate `tests/golden/*` from the Rust implementation.
use skill_sync::{detect, plan};
use skill_sync_core::fingerprint;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
  let out = root.join("tests/golden");
  fs::create_dir_all(&out)?;
  let home = tempfile::tempdir()?;
  let home = home.path();
  fs::create_dir_all(home.join(".cursor"))?;
  fs::create_dir_all(home.join(".claude"))?;
  let skill = home.join(".agents/skills/sample");
  fs::create_dir_all(skill.join("references"))?;
  fs::write(skill.join("SKILL.md"), "---\nname: sample\n---\nbody\n")?;
  fs::write(skill.join("references/rules.md"), "rules")?;
  let digest = fingerprint(&skill)?;
  let (roots, _) = detect(home, false);
  let report = plan(home, &roots, None)?;
  let digests = serde_json::json!({ "sample_skill_digest": digest });
  fs::write(
    out.join("digests.json"),
    serde_json::to_string_pretty(&digests)? + "\n",
  )?;
  let plan_golden = serde_json::json!({
      "skills": report.skills,
      "actions": report.actions,
      "conflicts": report.conflicts,
  });
  fs::write(
    out.join("plan-dry-run.json"),
    serde_json::to_string_pretty(&plan_golden)? + "\n",
  )?;
  eprintln!("Wrote goldens under {}", out.display());
  Ok(())
}
