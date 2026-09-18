#!/usr/bin/env python3
"""Sign the already-built app, notarize/staple, and create a notarized DMG.
Runs ONLY on a trusted macOS release runner. Never logs credential arguments.
"""
from __future__ import annotations
import base64
import json
import os
from pathlib import Path
import re
import secrets
import shlex
import shutil
import subprocess
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[1]
REQUIRED=('APPLE_CERTIFICATE','APPLE_CERTIFICATE_PASSWORD','APPLE_SIGNING_IDENTITY','APPLE_TEAM_ID',
          'NOTARY_KEY_P8','NOTARY_KEY_ID','NOTARY_ISSUER','RELEASE_VERSION','RELEASE_PLATFORM')
SECRET_NAMES={'APPLE_CERTIFICATE','APPLE_CERTIFICATE_PASSWORD','NOTARY_KEY_P8'}
SAFE_ENV={k:v for k,v in os.environ.items() if k not in SECRET_NAMES}

def command(*args: str, timeout: int=600) -> str:
    try:
        p=subprocess.run(args,cwd=ROOT,env=SAFE_ENV,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=timeout)
    except (OSError,subprocess.TimeoutExpired):
        raise RuntimeError('Signing tool failed or timed out: '+args[0]) from None
    if p.returncode:
        # Notary/security arguments include credentials. Do not echo commands or generic exceptions.
        raise RuntimeError('Signing tool failed: '+args[0]+' (exit '+str(p.returncode)+'). No release artifact is approved.')
    return p.stdout.strip()

def main():
    if sys.platform!='darwin': raise RuntimeError('This script requires macOS.')
    missing=[k for k in REQUIRED if not os.environ.get(k,'').strip()]
    if missing: raise RuntimeError('Missing signing configuration: '+', '.join(missing))
    identity=os.environ['APPLE_SIGNING_IDENTITY'];team=os.environ['APPLE_TEAM_ID']
    if not identity.startswith('Developer ID Application: ') or not identity.endswith('('+team+')'):
        raise RuntimeError('A matching Developer ID Application identity and team are required.')
    platform=os.environ['RELEASE_PLATFORM'];version=os.environ['RELEASE_VERSION']
    if platform not in ('macOS-arm64','macOS-x64') or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-(?:alpha|beta|rc)\.[0-9]+)?',version):
        raise RuntimeError('Invalid release identity.')
    if json.loads((ROOT/'src-tauri/tauri.conf.json').read_text(encoding="utf-8"))['version']!=version:
        raise RuntimeError('Release version mismatch.')
    before=shlex.split(command('security','list-keychains','-d','user'))
    default=command('security','default-keychain','-d','user').strip().strip('"')
    out=ROOT/'.release-assets/final';out.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='mpdv-signing-') as tmp:
        work=Path(tmp);keychain=work/'signing.keychain-db';password=secrets.token_urlsafe(40)
        p12=work/'signing.p12';key=work/'AuthKey.p8'
        p12.write_bytes(base64.b64decode(os.environ['APPLE_CERTIFICATE'],validate=True));p12.chmod(0o600)
        key.write_text(os.environ['NOTARY_KEY_P8'], encoding="utf-8");key.chmod(0o600)
        def notarize(path):
            result=json.loads(command('xcrun','notarytool','submit',str(path),'--key',str(key),
                                      '--key-id',os.environ['NOTARY_KEY_ID'],'--issuer',os.environ['NOTARY_ISSUER'],
                                      '--wait','--output-format','json',timeout=1800))
            if result.get('status')!='Accepted':
                raise RuntimeError('Notarization not accepted; submission '+str(result.get('id','unknown')))
            command('xcrun','stapler','staple',str(path));command('xcrun','stapler','validate',str(path))
        created=False
        try:
            command('security','create-keychain','-p',password,str(keychain));created=True
            command('security','unlock-keychain','-p',password,str(keychain))
            command('security','set-keychain-settings','-lut','21600',str(keychain))
            command('security','import',str(p12),'-k',str(keychain),'-P',os.environ['APPLE_CERTIFICATE_PASSWORD'],'-T','/usr/bin/codesign')
            command('security','set-key-partition-list','-S','apple-tool:,apple:,codesign:','-s','-k',password,str(keychain))
            command('security','list-keychains','-d','user','-s',str(keychain),*before)
            command('security','default-keychain','-d','user','-s',str(keychain))
            if '"'+identity+'"' not in command('security','find-identity','-v','-p','codesigning',str(keychain)):
                raise RuntimeError('Imported certificate does not match expected signing identity.')
            command('cargo','tauri','bundle','--bundles','app',timeout=900)
            apps=list((ROOT/'target/release/bundle/macos').glob('*.app'))
            if len(apps)!=1: raise RuntimeError('Expected exactly one app bundle.')
            app=apps[0]
            command('codesign','--verify','--deep','--strict','--verbose=2',str(app))
            appzip=work/'notary-app.zip'
            command('ditto','-c','-k','--keepParent','--sequesterRsrc',str(app),str(appzip))
            # ZIP files cannot be stapled: submit ZIP, staple its enclosed original app.
            result=json.loads(command('xcrun','notarytool','submit',str(appzip),'--key',str(key),
                '--key-id',os.environ['NOTARY_KEY_ID'],'--issuer',os.environ['NOTARY_ISSUER'],'--wait','--output-format','json',timeout=1800))
            if result.get('status')!='Accepted': raise RuntimeError('App notarization rejected; submission '+str(result.get('id','unknown')))
            command('xcrun','stapler','staple',str(app));command('xcrun','stapler','validate',str(app))
            command('spctl','--assess','--type','execute',str(app))
            stage=work/'image';stage.mkdir()
            command('ditto',str(app),str(stage/app.name))
            (stage/'Applications').symlink_to('/Applications')
            dmg=out/f'MPD_Viewer-v{version}-{platform}.dmg'
            command('hdiutil','create','-volname','MPD Viewer','-srcfolder',str(stage),'-format','UDZO',str(dmg))
            command('codesign','--force','--sign',identity,'--timestamp',str(dmg))
            notarize(dmg)
            command('codesign','--verify','--strict',str(dmg))
            print('Developer ID signing, notarization, and stapling passed for '+dmg.name)
        finally:
            # Restore the runner's original keychain configuration even on errors.
            try:
                command('security','list-keychains','-d','user','-s',*before)
                if default: command('security','default-keychain','-d','user','-s',default)
            finally:
                if created: command('security','delete-keychain',str(keychain))
if __name__=='__main__':
    try:main()
    except Exception as e:
        if isinstance(e,RuntimeError): print(str(e),file=sys.stderr)
        else: print('Signing stopped due to invalid input or an internal error; credentials suppressed.',file=sys.stderr)
        sys.exit(1)
