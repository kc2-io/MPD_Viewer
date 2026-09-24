import importlib.util
import json
import re
import io
import shutil
import sys
import tempfile
import tomllib
from contextlib import redirect_stdout
from unittest.mock import patch
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('configure',ROOT/'scripts/set-player-url.py')
configure=importlib.util.module_from_spec(spec)
spec.loader.exec_module(configure)

class ConfigurationTests(unittest.TestCase):
    def test_both_viewers_ship_without_feature_selected_builds(self):
        manifest=tomllib.loads((ROOT/'src-tauri/Cargo.toml').read_text(encoding='utf-8'))
        self.assertEqual(manifest['features']['default'],[])
        self.assertNotIn('twitch-page-viewer',manifest['features'])
        self.assertNotIn('twitch-embed-viewer',manifest['features'])
        source=(ROOT/'src-tauri/src/player.rs').read_text(encoding='utf-8')
        self.assertNotIn('compile_error!',source)
        self.assertIn('format!("twitch-page-{id}")',source)
        self.assertIn('const TWITCH_PAGE_ROOT: &str = "https://www.twitch.tv/";',source)

    def test_top_level_twitch_pages_receive_no_native_capability(self):
        capability=json.loads((ROOT/'src-tauri/capabilities/players.json').read_text())
        self.assertEqual(capability['windows'],['player-*'])
        self.assertNotIn('https://www.twitch.tv/*',capability['remote']['urls'])
        self.assertNotIn('twitch-page-*',capability['windows'])

    def test_manager_allows_html_ranking_drags(self):
        config=json.loads((ROOT/'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
        windows=config['app']['windows']
        manager=[window for window in windows if window['label']=='main']
        self.assertEqual(len(manager),1)
        # Tauri's native file-drop interception prevents HTML5 dragging on Windows.
        self.assertIs(manager[0].get('dragDropEnabled'),False)
        self.assertTrue(all(window.get('dragDropEnabled',True) for window in windows if window['label']!='main'))

    def test_registered_client_id_source_configuration(self):
        # Source guard only; Rust/SQLite runtime behavior has separate native tests.
        model=(ROOT/'src-tauri/src/model.rs').read_text()
        match=re.search(r'pub const DEFAULT_CLIENT_ID: &str = "([a-zA-Z0-9]+)";',model)
        self.assertIsNotNone(match)
        self.assertEqual(match.group(1),'ha94kk20cfu1tp74pgg8isgi88cpo7')
        self.assertNotIn('pub client_id:',model)
        controller=(ROOT/'src-tauri/src/controller.rs').read_text()
        self.assertIn('twitch.device_code(DEFAULT_CLIENT_ID)',controller)
        self.assertIn('twitch.complete_device(DEFAULT_CLIENT_ID, code)',controller)
    def test_https_wrapper(self):
        self.assertEqual(configure.validate_url('https://example.com/mpd/index.html'),'https://example.com/*')
    def test_https_directory(self):
        self.assertEqual(configure.validate_url('https://example.com/mpd/'),'https://example.com/*')
    def test_https_root_directory(self):
        self.assertEqual(configure.validate_url('https://parent.mpdviewer.com/'),
            'https://parent.mpdviewer.com/*')
    def test_compiled_origin_and_permission_stay_in_sync(self):
        config=json.loads((ROOT/'src-tauri/player-origin.json').read_text())
        capability=json.loads((ROOT/'src-tauri/capabilities/players.json').read_text())
        expected=[configure.LOCAL]
        if config['url'] is not None:
            expected.append(configure.validate_url(config['url']))
        self.assertEqual(capability['remote']['urls'],expected)
    def test_hosted_configuration_updates_both_files_without_manager_access(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            shutil.copytree(ROOT/'src-tauri/capabilities',root/'src-tauri/capabilities')
            manager=(root/'src-tauri/capabilities/manager.json').read_bytes()
            with patch.object(configure,'ROOT',root), patch.object(sys,'argv',
                    ['set-player-url.py','https://parent.mpdviewer.com/']), redirect_stdout(io.StringIO()):
                configure.main()
            config=json.loads((root/'src-tauri/player-origin.json').read_text())
            capability=json.loads((root/'src-tauri/capabilities/players.json').read_text())
            self.assertEqual(config,{'url':'https://parent.mpdviewer.com/'})
            self.assertEqual(capability['remote']['urls'],
                ['http://localhost:*/*','https://parent.mpdviewer.com/*'])
            self.assertEqual(capability['permissions'],['report-playback'])
            self.assertEqual((root/'src-tauri/capabilities/manager.json').read_bytes(),manager)
    def test_local_reset_removes_remote_host_permission(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            shutil.copytree(ROOT/'src-tauri/capabilities',root/'src-tauri/capabilities')
            with patch.object(configure,'ROOT',root), patch.object(sys,'argv',
                    ['set-player-url.py','--local']), redirect_stdout(io.StringIO()):
                configure.main()
            config=json.loads((root/'src-tauri/player-origin.json').read_text())
            capability=json.loads((root/'src-tauri/capabilities/players.json').read_text())
            self.assertIsNone(config['url'])
            self.assertEqual(capability['remote']['urls'],['http://localhost:*/*'])
    def test_invalid_origin_does_not_modify_configuration(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            shutil.copytree(ROOT/'src-tauri/capabilities',root/'src-tauri/capabilities')
            (root/'src-tauri/player-origin.json').write_text('{"url":null}\n')
            before={p.relative_to(root):p.read_bytes() for p in root.rglob('*') if p.is_file()}
            with patch.object(configure,'ROOT',root), patch.object(sys,'argv',
                    ['set-player-url.py','http://parent.mpdviewer.com/']), patch('sys.stderr',new=io.StringIO()):
                with self.assertRaises(SystemExit) as exc:
                    configure.main()
                self.assertEqual(exc.exception.code,2)
            after={p.relative_to(root):p.read_bytes() for p in root.rglob('*') if p.is_file()}
            self.assertEqual(before,after)
    def test_reject_insecure(self):
        with self.assertRaises(ValueError):configure.validate_url('http://example.com/index.html')
    def test_reject_credentials(self):
        with self.assertRaises(ValueError):configure.validate_url('https://user:secret@example.com/index.html')
    def test_reject_query_or_fragment(self):
        for suffix in ['?token=secret','#other']:
            with self.assertRaises(ValueError):configure.validate_url('https://example.com/index.html'+suffix)
    def test_reject_ambiguous_relative_asset_path(self):
        with self.assertRaises(ValueError):configure.validate_url('https://example.com/mpd')
    def test_remote_player_only_gets_advisory_reporting(self):
        value=json.loads((ROOT/'src-tauri/capabilities/players.json').read_text())
        self.assertEqual(value['permissions'],['report-playback'])
        self.assertEqual(value['windows'],['player-*'])
        self.assertFalse(value['local'])
    def test_manager_has_no_remote_origins(self):
        value=json.loads((ROOT/'src-tauri/capabilities/manager.json').read_text())
        self.assertEqual(value['windows'],['main'])
        self.assertNotIn('remote',value)
        self.assertEqual(value['permissions'],['manage'])

if __name__=='__main__':unittest.main(verbosity=2)
