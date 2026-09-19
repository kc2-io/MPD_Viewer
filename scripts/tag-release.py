#!/usr/bin/env python3
"""Create an annotated release tag; --push is an explicit publication trigger."""
import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('release_tools',ROOT/'scripts/release-tools.py')
tools=importlib.util.module_from_spec(spec);spec.loader.exec_module(tools)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('tag');p.add_argument('--push',action='store_true');a=p.parse_args()
    v=tools.parse_tag(a.tag)
    if v!=tools.version():raise ValueError('Set and commit matching workspace/Tauri versions first; do not rewrite versions inside CI.')
    tools.release_scope(v)
    if tools.run('git','status','--porcelain',capture=True):raise ValueError('Working tree must be clean.')
    if tools.run('git','branch','--show-current',capture=True)!='main':raise ValueError('Create release tags from main.')
    tools.require_lock();tools.require_release_pins()
    tools.run('git','fetch','origin','main','--tags')
    head=tools.run('git','rev-parse','HEAD',capture=True)
    if head!=tools.run('git','rev-parse','refs/remotes/origin/main',capture=True):raise ValueError('Local main must equal origin/main.')
    if subprocess.run(['git','show-ref','--verify','--quiet','refs/tags/'+a.tag],cwd=ROOT).returncode==0:
        raise ValueError('Tag already exists; release tags are never moved.')
    tools.run('git','tag','-a',a.tag,'-m','MPD Viewer '+a.tag)
    print('Created local annotated tag',a.tag)
    if a.push:
        tools.run('git','push','origin','refs/tags/'+a.tag)
        print('Pushed tag; GitHub workflow result and actual signing still need verification.')
    else:print('No tag was pushed. Push this exact tag explicitly to trigger a release.')
if __name__=='__main__':
    try:main()
    except (ValueError,OSError,subprocess.CalledProcessError) as e:
        print('Tagging stopped:',e,file=sys.stderr);sys.exit(1)
