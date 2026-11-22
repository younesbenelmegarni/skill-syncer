import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent


def _cli_binary() -> Path:
    env = os.environ.get('SKILL_SYNC_BIN', '').strip()
    if env:
        return Path(env)
    release = ROOT / 'target' / 'release' / 'skill-sync'
    if release.is_file():
        return release
    return ROOT / 'skill-sync'


CLI = _cli_binary()


class SkillSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='skill-sync-test-')
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        (self.home / '.cursor').mkdir()
        (self.home / '.claude').mkdir()

    def skill(self, root, name='sample', text='original', rule='mandatory rule'):
        p = self.home / root / name
        (p / 'references').mkdir(parents=True)
        (p / 'scripts').mkdir()
        (p / 'SKILL.md').write_text('---\nname: ' + name + '\n---\n' + text)
        (p / 'references/rules.md').write_text(rule)
        (p / 'CHANGELOG.md').write_text('original changelog')
        (p / 'scripts/run').write_text('#!/bin/sh\nexit 0\n')
        (p / 'scripts/run').chmod(0o755)
        return p

    def run_cli(self, *args, code=0):
        run = subprocess.run([str(CLI), '--home', str(self.home), '--json', *args], capture_output=True, text=True)
        self.assertEqual(run.returncode, code, run.stderr + run.stdout)
        return json.loads(run.stdout)

    def digest(self, path):
        run = subprocess.run([str(CLI), 'digest', str(path)], capture_output=True, text=True)
        self.assertEqual(run.returncode, 0, run.stderr + run.stdout)
        return run.stdout.strip()

    def harness_roots(self):
        return self.run_cli('dry-run')['harnesses']

    def test_full_directory_and_idempotency(self):
        p = self.skill('.agents/skills')
        r = self.run_cli()
        self.assertEqual(len(r['actions']), 2)
        for root in ['.cursor/skills', '.claude/skills']:
            link = self.home / root / 'sample'
            self.assertTrue(link.is_symlink())
            self.assertEqual((link / 'references/rules.md').read_text(), 'mandatory rule')
            self.assertEqual((link / 'CHANGELOG.md').read_text(), 'original changelog')
            self.assertTrue((link / 'scripts/run').stat().st_mode & 0o111)
        (self.home / '.cursor/skills/sample/references/rules.md').write_text('edited')
        self.assertEqual((p / 'references/rules.md').read_text(), 'edited')
        self.assertEqual(self.run_cli()['actions'], [])

    def test_dry_run_does_not_write(self):
        self.skill('.claude/skills')
        before = sorted(str(p.relative_to(self.home)) for p in self.home.rglob('*'))
        self.run_cli('dry-run')
        after = sorted(str(p.relative_to(self.home)) for p in self.home.rglob('*'))
        self.assertEqual(before, after)

    def test_identical_import_and_backups(self):
        self.skill('.claude/skills')
        self.skill('.cursor/skills')
        r = self.run_cli()
        self.assertTrue((self.home / '.agents/skills/sample/SKILL.md').is_file())
        manifest = json.loads((Path(r['backup']) / 'manifest.json').read_text())
        self.assertEqual(len(manifest), 2)
        for entry in manifest:
            self.assertTrue((Path(entry['backup']) / 'references/rules.md').is_file())

    def test_reference_only_conflict_leaves_all_copies(self):
        self.skill('.claude/skills', rule='A')
        self.skill('.cursor/skills', rule='B')
        self.skill('.agents/skills', 'safe')
        r = self.run_cli(code=2)
        self.assertIn('sample', r['conflicts'])
        self.assertFalse((self.home / '.claude/skills/sample').is_symlink())
        self.assertFalse((self.home / '.agents/skills/sample').exists())
        self.assertTrue((self.home / '.cursor/skills/safe').is_symlink())

    def test_explicit_promotion_preserves_losers(self):
        self.skill('.agents/skills', rule='old canonical')
        self.skill('.cursor/skills', rule='winner')
        self.skill('.claude/skills', rule='other copy')
        self.run_cli('--promote', 'cursor:sample', '--dry-run')
        r = self.run_cli('--promote', 'cursor:sample')
        self.assertEqual((self.home / '.agents/skills/sample/references/rules.md').read_text(), 'winner')
        manifest = json.loads((Path(r['backup']) / 'manifest.json').read_text())
        rules = {(Path(e['backup']) / 'references/rules.md').read_text() for e in manifest}
        self.assertEqual(rules, {'winner', 'old canonical', 'other copy'})
        self.assertEqual(self.run_cli()['actions'], [])

    def test_broken_link_preserved_and_reported(self):
        self.skill('.agents/skills')
        root = self.home / '.cursor/skills'
        root.mkdir()
        (root / 'sample').symlink_to('/nonexistent/skill-sync-test')
        r = self.run_cli(code=2)
        self.assertIn(str(root / 'sample'), r['broken_links'])
        self.assertEqual(str((root / 'sample').readlink()), '/nonexistent/skill-sync-test')
        self.assertFalse((self.home / '.claude/skills').exists())

    def test_managed_manifest_and_plugin_links_untouched(self):
        p = self.skill('.claude/skills')
        (p.parent / '.vendor-skills.json').write_text(json.dumps({'entries': {'sample': 'hash'}}))
        vendor = self.skill('.cursor/plugins/bundle', 'plugin')
        root = self.home / '.cursor/skills'
        root.mkdir()
        (root / 'plugin').symlink_to(vendor)
        r = self.run_cli()
        self.assertEqual(r['skills'], {})
        self.assertEqual(len(r['excluded_managed']), 2)
        self.assertFalse(p.is_symlink())

    def test_canonical_managed_name_is_not_propagated(self):
        self.skill('.agents/skills')
        p = self.skill('.claude/skills')
        (p.parent / '.vendor-skills.json').write_text(json.dumps({'entries': {'sample': 'hash'}}))
        r = self.run_cli(code=2)
        self.assertEqual(r['actions'], [])
        self.assertFalse(p.is_symlink())

    def test_external_symlink_import_keeps_target_contents(self):
        source = self.skill('personal')
        root = self.home / '.claude/skills'
        root.mkdir()
        (root / 'sample').symlink_to(source)
        self.run_cli()
        self.assertTrue((source / 'references/rules.md').is_file())
        self.assertTrue((self.home / '.agents/skills/sample/references/rules.md').is_file())

    def test_occupied_nonskill_not_overwritten(self):
        self.skill('.agents/skills')
        p = self.home / '.claude/skills/sample'
        p.mkdir(parents=True)
        (p / 'precious.txt').write_text('keep')
        r = self.run_cli(code=2)
        self.assertIn('sample', r['conflicts'])
        self.assertEqual((p / 'precious.txt').read_text(), 'keep')

    def test_executable_change_is_a_conflict(self):
        self.skill('.agents/skills')
        p = self.skill('.claude/skills')
        (p / 'scripts/run').chmod(0o644)
        self.assertIn('sample', self.run_cli(code=2)['conflicts'])

    def test_no_phantom_harness(self):
        self.skill('.agents/skills')
        self.assertEqual(set(self.run_cli()['harnesses']), {'agents', 'cursor', 'claude'})

    def test_root_alias_does_not_duplicate_actions(self):
        self.skill('.agents/skills')
        (self.home / '.claude/skills').mkdir()
        (self.home / '.cursor/skills').symlink_to(self.home / '.claude/skills')
        self.assertEqual(len(self.run_cli()['actions']), 1)
        self.assertEqual(self.run_cli()['actions'], [])

    def plugin_skill(self, version='old', name='plugin-skill', text='upstream'):
        return self.skill('.cursor/plugins/cache/cursor-public/pstack/' + version + '/skills', name, text)

    def test_plugin_import_and_links_without_touching_cache(self):
        source = self.plugin_skill()
        automation = self.skill('.cursor/plugins/cache/cursor-public/pstack/old/automations/benny/skills', 'automation-only')
        before = self.digest(source.parent.parent)
        r = self.run_cli()
        self.assertEqual(r['plugins']['pstack']['skills'], ['plugin-skill'])
        self.assertNotIn('automation-only', r['skills'])
        self.assertTrue((self.home / '.claude/skills/plugin-skill').is_symlink())
        self.assertEqual((self.home / '.agents/skills/plugin-skill/SKILL.md').read_text(), (source / 'SKILL.md').read_text())
        self.assertEqual(self.digest(source.parent.parent), before)
        self.assertEqual(self.run_cli('dry-run')['actions'], [])

    def test_newest_plugin_version_refreshes_canonical(self):
        import os
        old = self.plugin_skill('aaa', text='old')
        os.utime(old.parent.parent, (100, 100))
        self.run_cli()
        newer = self.plugin_skill('bbb', text='new')
        os.utime(newer.parent.parent, (200, 200))
        # Stale numeric marketplace id must not be selected, even if newer.
        self.skill('.cursor/plugins/cache/cursor-public/9717366/latest/skills', 'plugin-skill', 'stale alias')
        r = self.run_cli()
        self.assertEqual(r['plugins']['pstack']['version'], 'bbb')
        self.assertEqual(sum(a['op'] == 'import' for a in r['actions']), 1)
        self.assertIn('new', (self.home / '.agents/skills/plugin-skill/SKILL.md').read_text())
        self.assertEqual(self.run_cli('dry-run')['actions'], [])

    def test_plugin_divergent_copies_backed_up(self):
        self.plugin_skill()
        self.skill('.agents/skills', 'plugin-skill', 'canonical old')
        self.skill('.claude/skills', 'plugin-skill', 'divergent')
        r = self.run_cli()
        self.assertNotIn('plugin-skill', r['conflicts'])
        manifest = json.loads((Path(r['backup']) / 'manifest.json').read_text())
        self.assertEqual(len(manifest), 2)
        self.assertEqual({(Path(e['backup']) / 'SKILL.md').read_text().split('---')[-1].strip() for e in manifest}, {'canonical old', 'divergent'})
        self.assertTrue((self.home / '.claude/skills/plugin-skill').is_symlink())

    def test_plugin_broken_link_repaired(self):
        self.plugin_skill()
        root = self.home / '.claude/skills'
        root.mkdir()
        p = root / 'plugin-skill'
        p.symlink_to(self.home / '.cursor/plugins/cache/cursor-public/pstack/deleted/skills/plugin-skill')
        r = self.run_cli()
        self.assertEqual(r['broken_links'], [])
        self.assertEqual(r['repairable_broken_links'], [str(p)])
        self.assertEqual(p.resolve(), (self.home / '.agents/skills/plugin-skill').resolve())
        self.assertEqual(self.run_cli('dry-run')['actions'], [])

    def test_plugin_managed_destinations_are_skipped_not_written(self):
        self.plugin_skill()
        managed = self.skill('.claude/skills', 'plugin-skill', 'managed')
        (managed.parent / '.app-skills.json').write_text(json.dumps({'entries': {'plugin-skill': 'hash'}}))
        cursor_managed = self.skill('.cursor/skills-cursor', 'plugin-skill', 'cursor managed')
        before = (self.digest(managed), self.digest(cursor_managed))
        r = self.run_cli()
        self.assertTrue((self.home / '.cursor/skills/plugin-skill').is_symlink())
        self.assertIn(str(managed), r['skipped_managed_destinations'])
        self.assertEqual(before, (self.digest(managed), self.digest(cursor_managed)))
        self.assertEqual(self.run_cli('dry-run')['actions'], [])

    def test_plugin_real_nonskill_path_still_blocks(self):
        self.plugin_skill()
        p = self.home / '.claude/skills/plugin-skill'
        p.mkdir(parents=True)
        (p / 'precious.txt').write_text('keep')
        r = self.run_cli(code=2)
        self.assertIn('plugin-skill', r['conflicts'])
        self.assertEqual((p / 'precious.txt').read_text(), 'keep')

    def test_plugin_update_does_not_resolve_nonplugin_conflict(self):
        self.plugin_skill()
        self.skill('.claude/skills', rule='one')
        self.skill('.cursor/skills', rule='two')
        r = self.run_cli(code=2)
        self.assertIn('sample', r['conflicts'])
        self.assertFalse((self.home / '.claude/skills/sample').is_symlink())
        self.assertTrue((self.home / '.claude/skills/plugin-skill').is_symlink())

    def test_plugin_canonical_broken_link_repaired(self):
        self.plugin_skill()
        root = self.home / '.agents/skills'
        root.mkdir(parents=True)
        (root / 'plugin-skill').symlink_to('/missing/plugin')
        self.run_cli()
        self.assertFalse((root / 'plugin-skill').is_symlink())
        self.assertTrue((root / 'plugin-skill/SKILL.md').is_file())

    def test_canonical_plugin_symlink_becomes_owned_copy(self):
        source = self.plugin_skill()
        root = self.home / '.agents/skills'
        root.mkdir(parents=True)
        (root / 'plugin-skill').symlink_to(source)
        harness = self.home / '.claude/skills'
        harness.mkdir()
        (harness / 'plugin-skill').symlink_to(root / 'plugin-skill')
        self.run_cli()
        self.assertFalse((root / 'plugin-skill').is_symlink())
        self.assertEqual((harness / 'plugin-skill').resolve(), (root / 'plugin-skill').resolve())
        self.assertEqual(self.run_cli('dry-run')['actions'], [])

    def test_harness_root_pointing_inside_plugin_never_written(self):
        source = self.plugin_skill()
        (self.home / '.claude/skills').symlink_to(source.parent)
        before = self.digest(source.parent.parent)
        self.run_cli()
        self.assertEqual(self.digest(source.parent.parent), before)
        self.assertFalse(source.is_symlink())

    def test_auto_discovered_home_skill_root_gets_links(self):
        self.skill('.agents/skills')
        self.skill('.some-new-agent/skills')
        roots = self.harness_roots()
        self.assertIn('auto:.some-new-agent/skills', roots)
        self.run_cli()
        link = self.home / '.some-new-agent/skills/sample'
        self.assertTrue(link.is_symlink())
        self.assertEqual(link.resolve(), (self.home / '.agents/skills/sample').resolve())

    def test_plugin_cache_skills_not_auto_discovered(self):
        plugin_root = (self.home / '.cursor/plugins/cache/cursor-public/pstack/v1/skills/sample')
        plugin_root.parent.mkdir(parents=True)
        plugin_root.mkdir()
        (plugin_root / 'SKILL.md').write_text('---\nname: sample\n---\n')
        roots = self.harness_roots()
        self.assertFalse(any(k.startswith('auto:') and 'plugins' in k for k in roots))

    def test_t3_antigravity_config_skills_gets_links(self):
        self.skill('.agents/skills')
        instance = self.home / '.t3/userdata/providers/antigravity/abcd1234efgh5678'
        (instance / 'config/skills').mkdir(parents=True)
        roots = self.harness_roots()
        self.assertTrue(any(k.startswith('t3code-antigravity-') and k.endswith('-config') for k in roots))
        r = self.run_cli()
        link = instance / 'config/skills/sample'
        self.assertTrue(link.is_symlink())
        self.assertEqual(link.resolve(), (self.home / '.agents/skills/sample').resolve())

    def test_internal_links_are_preserved(self):
        p = self.skill('.claude/skills')
        (p / 'references/alias.md').symlink_to('rules.md')
        self.run_cli()
        dest = self.home / '.agents/skills/sample/references/alias.md'
        self.assertEqual(dest.readlink(), Path('rules.md'))
        self.assertEqual(dest.read_text(), 'mandatory rule')


if __name__ == '__main__':
    unittest.main()
