use skill_sync_core::fingerprint;
use std::fs;
use std::path::PathBuf;

#[test]
fn matches_golden_sample() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/digests.json");
  let data: serde_json::Value =
    serde_json::from_str(&fs::read_to_string(&root).expect("golden file")).unwrap();
  let expected = data["sample_skill_digest"].as_str().expect("digest");
  // Golden digest was recorded from a disposable home; regenerate if SKILL.md body changes.
  // This test only guards the repr+hash pipeline against accidental drift in CI.
  let dir = tempfile::tempdir().unwrap();
  let skill = dir.path().join("sample");
  fs::create_dir_all(skill.join("references")).unwrap();
  fs::write(skill.join("SKILL.md"), "---\nname: sample\n---\nbody\n").unwrap();
  fs::write(skill.join("references/rules.md"), "rules").unwrap();
  let got = fingerprint(&skill).expect("fingerprint");
  assert_ne!(got.len(), 0);
  assert_eq!(got.len(), 64);
  if got != expected {
    eprintln!("regenerate goldens: ./benchmarks/generate_goldens.sh");
  }
  assert_eq!(got, expected);
}
