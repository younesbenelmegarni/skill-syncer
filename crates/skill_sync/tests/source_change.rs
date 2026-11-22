use skill_sync::{apply, detect, plan};
use std::fs;
use std::path::{Path, PathBuf};

fn claude_sample(base: &Path) -> PathBuf {
  let skill = base.join(".claude/skills/sample");
  fs::create_dir_all(skill.join("references")).unwrap();
  fs::write(skill.join("SKILL.md"), "---\nname: sample\n---\noriginal").unwrap();
  fs::write(skill.join("references/rules.md"), "mandatory rule").unwrap();
  skill
}

#[test]
fn concurrent_source_change_aborts_apply() {
  let tmp = tempfile::tempdir().unwrap();
  let home = tmp.path().to_path_buf();
  fs::create_dir_all(home.join(".cursor")).unwrap();
  let skill = claude_sample(&home);
  let state = home.join(".local/state/skill-sync");
  let (roots, _) = detect(&home, false);
  let report = plan(&home, &roots, None).unwrap();
  fs::write(skill.join("references/rules.md"), "changed while planning").unwrap();
  let err = apply(&report, &state).unwrap_err();
  assert!(err.to_string().contains("Source changed"), "{}", err);
  assert!(!home.join(".agents/skills/sample").exists());
}
