"""Offline release helper/security checks. These do not execute GitHub, Rust or signing tools."""
import importlib.util
import json
import os
import shutil
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
  (self.root/'.github').mkdir()
  (self.root/'.github/release-policy.json').write_text(json.dumps({'repository':'kc2-io/MPD_Viewer','visibility':'public'}))
  (self.root/'.github/release-scope.json').write_text('{"scope":"full"}')
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
 def test_unapproved_private_repo_never_releases(self):
  with patch.dict(os.environ,{'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'true','GITHUB_REPOSITORY':'kc2-io/MPD_Viewer'},clear=True),self.assertRaisesRegex(ValueError,'visibility'):r.gate()
 def test_approved_public_repository_policy(self):
  r.require_repository_policy('kc2-io/MPD_Viewer',False)
 def test_fork_identity_rejected(self):
  with self.assertRaisesRegex(ValueError,'identity'):r.require_repository_policy('other/MPD_Viewer',False)
 def test_unknown_visibility_rejected(self):
  for value in (None,'false',0):
   with self.subTest(value=value),self.assertRaisesRegex(ValueError,'visibility'):r.require_repository_policy('kc2-io/MPD_Viewer',value)
 def test_missing_event_visibility_rejected(self):
  with patch.dict(os.environ,{'RELEASES_ENABLED':'true','GITHUB_REPOSITORY':'kc2-io/MPD_Viewer'},clear=True),self.assertRaisesRegex(ValueError,'visibility'):r.gate()
 def test_invalid_committed_policy_rejected(self):
  (self.root/'.github/release-policy.json').write_text('{"repository":"kc2-io/MPD_Viewer","visibility":"any"}')
  with self.assertRaisesRegex(ValueError,'policy'):r.release_policy()
 def test_reviewed_public_prerelease_still_checks_tag_ancestry(self):
  env={'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'false','GITHUB_REPOSITORY':'kc2-io/MPD_Viewer','GITHUB_EVENT_NAME':'push','GITHUB_REF_TYPE':'tag','RELEASE_TAG':'v1.2.3-rc.1'}
  def git(*args,**kwargs):return 'tag' if args[1]=='cat-file' else 'a'*40
  with patch.dict(os.environ,env,clear=True),patch.object(r,'require_lock'),patch.object(r,'require_release_pins'),patch.object(r,'run',side_effect=git) as run:
   r.gate();run.assert_any_call('git','merge-base','--is-ancestor','HEAD','refs/remotes/origin/main')
 def test_missing_committed_policy_rejected(self):
  (self.root/'.github/release-policy.json').unlink()
  with self.assertRaises(OSError):r.require_repository_policy('kc2-io/MPD_Viewer',False)
 def test_malformed_committed_policy_rejected(self):
  for data in ('null','[]','{}','{broken'):
   with self.subTest(data=data):
    (self.root/'.github/release-policy.json').write_text(data)
    with self.assertRaises(ValueError):r.require_repository_policy('kc2-io/MPD_Viewer',False)
 def test_pr_cannot_sign(self):
  with patch.dict(os.environ,{'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'false','GITHUB_REPOSITORY':'kc2-io/MPD_Viewer','GITHUB_EVENT_NAME':'pull_request','GITHUB_REF_TYPE':'branch'},clear=True),self.assertRaisesRegex(ValueError,'pushed'):r.gate()
 def test_stable_needs_its_own_enable(self):
  (self.root/'Cargo.toml').write_text('[workspace.package]\nversion="1.2.3"\n');(self.root/'src-tauri/tauri.conf.json').write_text('{"version":"1.2.3"}')
  env={'RELEASES_ENABLED':'true','REPOSITORY_PRIVATE':'false','GITHUB_REPOSITORY':'kc2-io/MPD_Viewer','GITHUB_EVENT_NAME':'push','GITHUB_REF_TYPE':'tag','RELEASE_TAG':'v1.2.3'}
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
  text=(ROOT/'.github/workflows/release.yml').read_text();self.assertNotIn('continue-on-error',text);self.assertIn('needs: [gate, build, windows, macos, macos_alpha, linux]',text);self.assertIn("needs.windows.result == 'success'",text);self.assertIn("needs.build.result == 'success'",text);self.assertIn("needs.macos.result == 'skipped'",text);self.assertIn("needs.macos_alpha.result == 'success'",text);self.assertIn("needs.linux.result == 'skipped'",text);self.assertIn('!cancelled()',text)
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
 def test_publication_rejects_changed_live_visibility(self):
  env={'RELEASE_VERSION':'1.2.3-rc.1','RELEASE_TAG':'v1.2.3-rc.1','GH_REPO':'kc2-io/MPD_Viewer'}
  with patch.dict(os.environ,env,clear=True),patch.object(r,'run',return_value='a'*40),patch.object(r,'gh_json',return_value={'full_name':'kc2-io/MPD_Viewer','private':True}),self.assertRaisesRegex(ValueError,'visibility'):
   r.publish()
 def test_visibility_change_after_upload_leaves_draft_unpublished(self):
  env={'RELEASE_VERSION':'1.2.3-rc.1','RELEASE_TAG':'v1.2.3-rc.1','GH_REPO':'kc2-io/MPD_Viewer'}
  out=r.final_dir()
  for name in r.asset_names('1.2.3-rc.1')[:5]:(out/name).write_bytes(b'fixture binary')
  (self.root/'Cargo.lock').write_bytes(b'fixture lock')
  def command(*args,**kwargs):
   if args[:2]==('git','rev-parse'):return 'a'*40
   if args[:2]==('git','archive'):
    target=next(a.removeprefix('--output=') for a in args if a.startswith('--output='));Path(target).write_bytes(b'fixture source')
   if args[:3]==('gh','release','download'):
    dest=Path(args[args.index('--dir')+1])
    for path in out.iterdir():shutil.copy2(path,dest/path.name)
   return ''
  responses=[{'full_name':'kc2-io/MPD_Viewer','private':False},[],{'full_name':'kc2-io/MPD_Viewer','private':True}]
  with patch.dict(os.environ,env,clear=True),patch.object(r,'run',side_effect=command) as run,patch.object(r,'gh_json',side_effect=responses),patch.object(r,'live_commit',return_value='a'*40):
   with self.assertRaisesRegex(ValueError,'visibility'):r.publish()
   self.assertTrue(any(c.args[:3]==('gh','release','create') for c in run.call_args_list))
   self.assertFalse(any(c.args[:3]==('gh','release','edit') for c in run.call_args_list))
 def alpha_scope(self):
  v='0.1.0-alpha.1'
  (self.root/'Cargo.toml').write_text(f'[workspace.package]\nversion="{v}"\n')
  (self.root/'src-tauri/tauri.conf.json').write_text(json.dumps({'version':v}))
  (self.root/'.github/release-scope.json').write_text('{"scope":"windows-alpha"}')
  return v
 def test_windows_scope_only_accepts_alpha(self):
  self.alpha_scope()
  self.assertEqual(r.release_scope('0.1.0-alpha.1'),'windows-alpha')
  for v in ['0.1.0','0.1.0-beta.1','0.1.0-rc.1','0.1.0-alpha.01','0.1.0-alpha.1+foo']:
   with self.subTest(v=v),self.assertRaises(ValueError):r.release_scope(v)
 def test_release_scope_fails_closed(self):
  for value in ['{}','null','{"scope":"unsigned"}','{"scope":"windows-alpha","extra":true}']:
   (self.root/'.github/release-scope.json').write_text(value)
   with self.subTest(value=value),self.assertRaises(ValueError):r.release_scope('0.1.0-alpha.1')
  (self.root/'.github/release-scope.json').unlink()
  with self.assertRaises(OSError):r.release_scope('0.1.0-alpha.1')
 def multiplatform_alpha_scope(self):
  v='0.1.0-alpha.7'
  (self.root/'Cargo.toml').write_text(f'[workspace.package]\nversion="{v}"\n')
  (self.root/'src-tauri/tauri.conf.json').write_text(json.dumps({'version':v}))
  (self.root/'.github/release-scope.json').write_text('{"scope":"multiplatform-alpha"}')
  return v
 def test_multiplatform_scope_only_accepts_alpha(self):
  self.multiplatform_alpha_scope()
  self.assertEqual(r.release_scope('0.1.0-alpha.7'),'multiplatform-alpha')
  for v in ['0.1.0','0.1.0-beta.1','0.1.0-rc.1','0.1.0-alpha.01','0.1.0-alpha.7+foo']:
   with self.subTest(v=v),self.assertRaises(ValueError):r.release_scope(v)
 def test_multiplatform_alpha_assets_state_unsigned_platforms(self):
  v=self.multiplatform_alpha_scope();names=r.release_assets(v,r.release_scope(v))
  self.assertEqual(names,[f'MPD_Viewer-v{v}-Windows-x64.zip',
                          f'MPD_Viewer-v{v}-macOS-arm64-UNSIGNED.zip',
                          f'MPD_Viewer-v{v}-macOS-x64-UNSIGNED.zip',
                          f'MPD_Viewer-v{v}-Linux-x64-UNSIGNED.deb',
                          f'MPD_Viewer-v{v}-Linux-x64-UNSIGNED.AppImage',
                          f'MPD_Viewer-v{v}-source.zip',f'MPD_Viewer-v{v}-player-sources.zip'])
 def test_unsigned_macos_package_is_alpha_scoped_and_uses_ditto(self):
  v=self.multiplatform_alpha_scope();app=self.root/'target/release/bundle/macos/MPD Viewer.app';app.mkdir(parents=True)
  with patch.dict(os.environ,{'RELEASE_VERSION':v}),patch.object(r.sys,'platform','darwin'),patch.object(r,'run') as run:
   r.package_macos_unsigned('macOS-arm64')
  self.assertEqual(run.call_args.args[:6],('ditto','-c','-k','--keepParent','--sequesterRsrc',str(app)))
  self.assertTrue(str(run.call_args.args[-1]).endswith(f'MPD_Viewer-v{v}-macOS-arm64-UNSIGNED.zip'))
 def test_alpha_has_exact_windows_and_source_set(self):
  v=self.alpha_scope();names=r.release_assets(v,r.release_scope(v))
  self.assertEqual(names,[f'MPD_Viewer-v{v}-Windows-x64.zip',f'MPD_Viewer-v{v}-source.zip',f'MPD_Viewer-v{v}-player-sources.zip'])
  self.assertEqual(r.release_assets('1.2.3-rc.1','full'),r.asset_names('1.2.3-rc.1'))
  with self.assertRaises(ValueError):r.release_assets('1.2.3','windows-alpha')
 def alpha_publish(self,extra=None,missing=False,tamper=False,changed_tag=False,existing=False):
  v=self.alpha_scope();out=r.final_dir()
  if not missing:(out/f'MPD_Viewer-v{v}-Windows-x64.zip').write_bytes(b'SIGNED-BINARY-TEST-FIXTURE')
  if extra:(out/extra).write_bytes(b'unexpected')
  (self.root/'Cargo.lock').write_bytes(b'fixture lock')
  env={'RELEASE_VERSION':v,'RELEASE_TAG':'v'+v,'GH_REPO':'kc2-io/MPD_Viewer'}
  def command(*args,**kwargs):
   if args[:2]==('git','rev-parse'):return 'a'*40
   if args[:2]==('git','archive'):
    target=next(a.removeprefix('--output=') for a in args if a.startswith('--output='));Path(target).write_bytes(b'fixture source')
   if args[:3]==('gh','release','download'):
    dest=Path(args[args.index('--dir')+1])
    for p in out.iterdir():shutil.copy2(p,dest/p.name)
    if tamper:(dest/'BUILD-METADATA.json').write_bytes(b'tampered')
   return ''
  responses=[{'full_name':'kc2-io/MPD_Viewer','private':False},[[{'tag_name':'v'+v}]] if existing else [],{'full_name':'kc2-io/MPD_Viewer','private':False}]
  with patch.dict(os.environ,env,clear=True),patch.object(r,'run',side_effect=command) as run,patch.object(r,'gh_json',side_effect=responses),patch.object(r,'live_commit',side_effect=['a'*40,('b' if changed_tag else 'a')*40]):
   if extra or missing or tamper or changed_tag or existing:
    with self.assertRaises(ValueError):r.publish()
    self.assertFalse(any(c.args[:3]==('gh','release','edit') for c in run.call_args_list))
   else:
    r.publish()
    create=next(c.args for c in run.call_args_list if c.args[:3]==('gh','release','create'))
    self.assertIn('--prerelease',create);self.assertIn('--draft',create);self.assertIn('--verify-tag',create)
    edit=next(c.args for c in run.call_args_list if c.args[:3]==('gh','release','edit'))
    self.assertIn('--latest=false',edit)
    meta=json.loads((out/'BUILD-METADATA.json').read_text())
    self.assertEqual(meta['scope'],'windows-alpha');self.assertEqual(set(meta['signing']),{'windows'})
    self.assertEqual(set(meta['assets']),set(r.release_assets(v,'windows-alpha')))
    self.assertEqual(len(list(out.iterdir())),5)
 def test_alpha_publication_verified_prerelease_not_latest(self):self.alpha_publish()
 def test_alpha_missing_windows_rejected(self):self.alpha_publish(missing=True)
 def test_alpha_unexpected_macos_rejected(self):self.alpha_publish(extra='MPD_Viewer-v0.1.0-alpha.1-macOS-arm64.dmg')
 def test_alpha_unexpected_unsigned_rejected(self):self.alpha_publish(extra='UNSIGNED.zip')
 def test_alpha_corrupt_download_never_publishes(self):self.alpha_publish(tamper=True)
 def test_alpha_changed_tag_never_publishes(self):self.alpha_publish(changed_tag=True)
 def test_alpha_existing_release_never_overwritten(self):self.alpha_publish(existing=True)
 def test_multiplatform_alpha_packages_linux_as_explicitly_unsigned(self):
  v=self.multiplatform_alpha_scope();bundle=self.root/'target/release/bundle';bundle.mkdir(parents=True)
  (bundle/'mpd-viewer.deb').write_bytes(b'deb');(bundle/'mpd-viewer.AppImage').write_bytes(b'appimage')
  with patch.dict(os.environ,{'RELEASE_VERSION':v}):r.package_linux()
  self.assertEqual((r.final_dir()/f'MPD_Viewer-v{v}-Linux-x64-UNSIGNED.deb').read_bytes(),b'deb')
  self.assertEqual((r.final_dir()/f'MPD_Viewer-v{v}-Linux-x64-UNSIGNED.AppImage').read_bytes(),b'appimage')
 def test_multiplatform_alpha_publication_records_trust_state(self):
  v=self.multiplatform_alpha_scope();out=r.final_dir()
  for name in r.multiplatform_alpha_asset_names(v)[:5]:(out/name).write_bytes(b'platform fixture')
  (self.root/'Cargo.lock').write_bytes(b'fixture lock')
  env={'RELEASE_VERSION':v,'RELEASE_TAG':'v'+v,'GH_REPO':'kc2-io/MPD_Viewer'}
  def command(*args,**kwargs):
   if args[:2]==('git','rev-parse'):return 'a'*40
   if args[:2]==('git','archive'):
    target=next(a.removeprefix('--output=') for a in args if a.startswith('--output='));Path(target).write_bytes(b'fixture source')
   if args[:3]==('gh','release','download'):
    dest=Path(args[args.index('--dir')+1])
    for p in out.iterdir():shutil.copy2(p,dest/p.name)
   return ''
  responses=[{'full_name':'kc2-io/MPD_Viewer','private':False},[],{'full_name':'kc2-io/MPD_Viewer','private':False}]
  with patch.dict(os.environ,env,clear=True),patch.object(r,'run',side_effect=command) as run,patch.object(r,'gh_json',side_effect=responses),patch.object(r,'live_commit',return_value='a'*40):r.publish()
  meta=json.loads((out/'BUILD-METADATA.json').read_text())
  self.assertEqual(meta['scope'],'multiplatform-alpha')
  self.assertIn('UNSIGNED',meta['signing']['macos']);self.assertIn('No OS-native signature',meta['signing']['linux'])
  self.assertEqual(set(meta['assets']),set(r.multiplatform_alpha_asset_names(v)))
  self.assertEqual(len(list(out.iterdir())),9)
  self.assertIn('UNSIGNED',(self.root/'.release-work/notes.md').read_text())
  create=next(c.args for c in run.call_args_list if c.args[:3]==('gh','release','create'))
  self.assertIn('--prerelease',create)

if __name__=='__main__':unittest.main(verbosity=2)
