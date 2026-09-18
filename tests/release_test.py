"""Offline release helper/security checks. These do not execute GitHub, Rust or signing tools."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile
ROOT=Path(__file__).resolve().parents[1]
def load(name,path):
 s=importlib.util.spec_from_file_location(name,ROOT/path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
r=load('rt','scripts/release-tools.py');pa=load('pa','scripts/pin-actions.py');boot=load('boot','scripts/bootstrap-github.py')
class ReleaseTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name);(self.root/'src-tauri').mkdir()
  (self.root/'Cargo.toml').write_text('[workspace.package]\nversion="1.2.3-rc.1"\n')
  (self.root/'src-tauri/tauri.conf.json').write_text('{"version":"1.2.3-rc.1"}')
  self.p=patch.object(r,'ROOT',self.root);self.p.start()
 def tearDown(self):self.p.stop();self.temp.cleanup()
 def test_supported_tags(self):
  for t in ('v0.1.0','v1.2.3-rc.1','v1.2.3-alpha.0','v1.2.3-beta.9'):self.assertEqual(r.parse_tag(t),t[1:])
 def test_reject_unsafe_or_ambiguous_tags(self):
  for t in ('1.0.0','v01.0.0','v1.0.0;echo bad','v1.0.0\n','v1.0.0-rc.01','v1.0.0+build','../../main','v1.2'):
   with self.subTest(t=t),self.assertRaises(ValueError):r.parse_tag(t)
 def test_matching_versions(self):self.assertEqual(r.version(),'1.2.3-rc.1')
 def test_version_mismatch(self):
  (self.root/'src-tauri/tauri.conf.json').write_text('{"version":"1.2.4"}')
  with self.assertRaises(ValueError):r.version()
 def test_missing_lock_stops_release(self):
  with self.assertRaises(ValueError):r.require_lock()
 def test_empty_lock_stops_release(self):
  (self.root/'Cargo.lock').write_text('version=4\n')
  with self.assertRaises(ValueError):r.require_lock()
 def test_untracked_lock_stops_release(self):
  (self.root/'Cargo.lock').write_text('version=4\n[[package]]\nname="a"\nversion="1.0.0"\n')
  with patch.object(r,'run',side_effect=ValueError('untracked')),self.assertRaises(ValueError):r.require_lock()
 def test_disable_is_default(self):
  with patch.dict(os.environ,{},clear=True),self.assertRaisesRegex(ValueError,'disabled'):r.gate()
 def test_public_repo_never_releases(self):
  with patch.dict(os.environ,{'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'false'},clear=True),self.assertRaisesRegex(ValueError,'PRIVATE'):r.gate()
 def test_pr_cannot_sign(self):
  with patch.dict(os.environ,{'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'true','GITHUB_EVENT_NAME':'pull_request','GITHUB_REF_TYPE':'branch'},clear=True),self.assertRaisesRegex(ValueError,'pushed'):r.gate()
 def test_stable_needs_its_own_enable(self):
  (self.root/'Cargo.toml').write_text('[workspace.package]\nversion="1.2.3"\n');(self.root/'src-tauri/tauri.conf.json').write_text('{"version":"1.2.3"}')
  env={'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'true','GITHUB_EVENT_NAME':'push','GITHUB_REF_TYPE':'tag','RELEASE_TAG':'v1.2.3'}
  with patch.dict(os.environ,env,clear=True),self.assertRaisesRegex(ValueError,'Stable'):r.gate()
 def test_configuration_errors_never_include_values(self):
  with patch.dict(os.environ,{'TOKEN':'not-for-logs'},clear=True):
   with self.assertRaises(ValueError) as e:r.require_env(['TOKEN','MISSING'])
   self.assertNotIn('not-for-logs',str(e.exception));self.assertIn('MISSING',str(e.exception))
 def test_asset_names_unique_and_versioned(self):
  names=r.asset_names('1.2.3-rc.1');self.assertEqual(len(set(names)),7);self.assertTrue(all('v1.2.3-rc.1' in n for n in names))
 def test_sha256(self):
  p=self.root/'a';p.write_bytes(b'abc');self.assertEqual(r.digest(p),'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
 def test_windows_zip_preserves_input_bytes(self):
  p=self.root/'.release-work/windows/mpd-tabber.exe';p.parent.mkdir(parents=True);p.write_bytes(b'TEST FIXTURE NOT A SIGNED EXECUTABLE');(self.root/'LICENSE').write_text('fixture')
  with patch.dict(os.environ,{'RELEASE_VERSION':'1.2.3-rc.1'}):r.package_windows()
  with zipfile.ZipFile(r.final_dir()/r.asset_names('1.2.3-rc.1')[0]) as z:self.assertEqual(z.read('MPD_Viewer.exe'),p.read_bytes())
 def test_linux_refuses_missing_bundle(self):
  with patch.dict(os.environ,{'RELEASE_VERSION':'1.2.3-rc.1'}),self.assertRaises(ValueError):r.package_linux()
 def test_linux_rejects_ambiguous_bundle(self):
  p=self.root/'target/release/bundle';p.mkdir(parents=True);(p/'a.deb').write_bytes(b'1');(p/'b.deb').write_bytes(b'2')
  with patch.dict(os.environ,{'RELEASE_VERSION':'1.2.3-rc.1'}),self.assertRaises(ValueError):r.package_linux()
 def test_ci_artifact_explicitly_unsigned(self):
  p=self.root/'target/release';p.mkdir(parents=True);(p/'mpd-tabber.exe').write_bytes(b'fixture');r.stage_ci('Windows-x64')
  with zipfile.ZipFile(self.root/'.release-assets/ci/MPD_Viewer-Windows-x64-UNSIGNED.zip') as z:self.assertIn(b'UNSIGNED',z.read('READ-ME-FIRST.txt'))
 def test_action_ref_resolution_is_from_tag(self):
  with patch.object(pa,'api',return_value={'object':{'type':'commit','sha':'a'*40}}) as get:
   self.assertEqual(pa.resolve('actions/checkout','v7'),'a'*40);get.assert_called_once_with('repos/actions/checkout/git/ref/tags/v7')
 def test_annotated_action_tag_peels(self):
  responses=[{'object':{'type':'tag','sha':'b'*40}},{'object':{'type':'commit','sha':'a'*40}}]
  with patch.object(pa,'api',side_effect=responses):self.assertEqual(pa.resolve('actions/checkout','v7'),'a'*40)
 def test_action_rejects_noncommit(self):
  with patch.object(pa,'api',return_value={'object':{'type':'blob','sha':'a'*40}}),self.assertRaises(ValueError):pa.resolve('actions/checkout','v7')
 def test_yaml_workflow_requires_pins(self):
  workflows=self.root/'.github/workflows';workflows.mkdir(parents=True)
  (workflows/'extra.yaml').write_text('steps:\n  - uses: actions/checkout@v7\n')
  with patch.object(pa,'ROOT',self.root),patch('sys.argv',['pin-actions.py','--check']),self.assertRaisesRegex(ValueError,'not pinned'):pa.main()
 def test_unsupported_action_syntax_fails_closed(self):
  workflows=self.root/'.github/workflows';workflows.mkdir(parents=True)
  for line in ('  - uses: "actions/checkout@v7"', '  - uses: actions/checkout/subpath@v7', '  - { uses: actions/checkout@v7 }', '  - "uses": actions/checkout@v7', '  - uses:'):
   with self.subTest(line=line):
    (workflows/'extra.yml').write_text('steps:\n'+line+'\n')
    with patch.object(pa,'ROOT',self.root),patch('sys.argv',['pin-actions.py','--check']),self.assertRaisesRegex(ValueError,'unsupported uses syntax'):pa.main()
 def test_native_ci_has_no_signing_or_oidc(self):
  text=(ROOT/'.github/workflows/ci.yml').read_text();self.assertNotIn('secrets.',text);self.assertNotIn('id-token: write',text);self.assertNotIn('environment:',text)
 def test_release_has_no_unsafe_pr_trigger(self):
  text=(ROOT/'.github/workflows/release.yml').read_text();self.assertNotIn('pull_request_target',text);self.assertNotIn('workflow_run:',text)
 def test_signing_failure_not_ignored(self):
  text=(ROOT/'.github/workflows/release.yml').read_text();self.assertNotIn('continue-on-error',text);self.assertIn('needs: [gate, windows, macos, linux]',text)
 def test_only_signer_can_request_oidc(self):
  text=(ROOT/'.github/workflows/release.yml').read_text();self.assertEqual(text.count('id-token: write'),1)
 def test_preserve_import_snapshot(self):
  meta=json.loads((ROOT/'web/parent.mpdviewer.com/SOURCE.json').read_text())
  actual=r.digest(ROOT/'web/parent.mpdviewer.com/index.html')
  self.assertIn(actual,json.dumps(meta))
 def test_no_unsafe_publish_paths(self):
  text=(ROOT/'scripts/release-tools.py').read_text();self.assertIn('--verify-tag',text);self.assertIn('--draft',text);self.assertNotIn('--clobber',text)
 def test_bootstrap_only_private_creation(self):
  text=(ROOT/'scripts/bootstrap-github.py').read_text();self.assertIn("'--private'",text);self.assertNotIn("'--public'",text);self.assertNotIn('gh secret',text)
 def test_no_workflow_deploys_the_parent(self):
  for p in (ROOT/'.github/workflows').glob('*.yml'):
   self.assertNotIn('wrangler',p.read_text());self.assertNotIn('cloudflare',p.read_text())
 def test_publication_rechecks_live_private_status(self):
  env={'RELEASE_VERSION':'1.2.3-rc.1','RELEASE_TAG':'v1.2.3-rc.1','GH_REPO':'kc2-io/MPD_Viewer'}
  with patch.dict(os.environ,env,clear=True),patch.object(r,'run',return_value='a'*40),patch.object(r,'gh_json',return_value={'private':False}),self.assertRaisesRegex(ValueError,'visibility'):
   r.publish()
if __name__=='__main__':unittest.main(verbosity=2)
