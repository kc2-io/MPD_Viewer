#!/usr/bin/env python3
"""Resolve official Actions release tags to commit SHAs, or check existing pins."""
from pathlib import Path
import argparse
import json
import re
import subprocess
import sys
from urllib.parse import quote
ROOT = Path(__file__).resolve().parents[1]
PATTERN = re.compile(r'^(\s*(?:-\s*)?uses:\s*)([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)@([^\s#]+)([^\n]*)$', re.M)
SHA = re.compile(r'[0-9a-f]{40}')
ALLOW = {'actions/checkout', 'actions/setup-node', 'actions/setup-python', 'actions/upload-artifact',
         'actions/download-artifact', 'azure/login', 'azure/artifact-signing-action'}

def api(path):
    return json.loads(subprocess.run(['gh','api',path],check=True,text=True,stdout=subprocess.PIPE).stdout)

def resolve(repo, ref):
    obj=api(f'repos/{repo}/git/ref/tags/{quote(ref,safe="")}')['object']
    for _ in range(6):
        if obj['type']=='commit' and SHA.fullmatch(obj['sha']): return obj['sha']
        if obj['type']!='tag': raise ValueError('Reference did not resolve to an annotated tag or commit.')
        obj=api(f'repos/{repo}/git/tags/{obj["sha"]}')['object']
    raise ValueError('Excessive tag indirection.')

def main():
    p=argparse.ArgumentParser(description=__doc__)
    g=p.add_mutually_exclusive_group(required=True);g.add_argument('--write',action='store_true');g.add_argument('--check',action='store_true')
    args=p.parse_args(); changes={};pins={};unresolved=[]
    for path in sorted(p for p in (ROOT/'.github/workflows').iterdir() if p.suffix in ('.yml', '.yaml')):
        text=path.read_text(encoding="utf-8")
        # Accept only the simple block-style uses syntax handled below. Fail closed
        # for quoted, flow-style, subpath or multiline uses rather than skipping it.
        for number, line in enumerate(text.splitlines(), 1):
            if line.lstrip().startswith('#'):
                continue
            if re.search(r"\buses['\"]?\s*:", line) and not PATTERN.fullmatch(line):
                raise ValueError(f'{path.name}:{number}: unsupported uses syntax; use an unquoted owner/repo@ref block entry.')
        def replacement(m):
            prefix,repo,ref,trailer=m.groups()
            if repo.lower() not in ALLOW: raise ValueError('Review an unexpected action before using it: '+repo)
            if SHA.fullmatch(ref): return m[0]
            if args.check: unresolved.append(f'{path.name}: {repo}@{ref}');return m[0]
            key=repo+'@'+ref
            if key not in pins: pins[key]=resolve(repo,ref)
            return prefix+repo+'@'+pins[key]+' # '+ref
        changes[path]=PATTERN.sub(replacement,text)
    if unresolved: raise ValueError('Actions are not pinned to commit SHAs: '+', '.join(unresolved))
    if args.write:
        # All lookups finish before modifying any workflow.
        for path,text in changes.items(): path.write_text(text, encoding="utf-8")
        file=ROOT/'.github/action-pins.json'
        prior=json.loads(file.read_text(encoding="utf-8")) if file.exists() else {}
        prior.update(pins);file.write_text(json.dumps(prior,indent=2,sort_keys=True)+'\n', encoding="utf-8")
        print('Resolved',len(pins),'action release tags; review and commit all changed files.')
    else: print('Every external workflow action uses a full commit SHA.')
if __name__=='__main__':
    try:main()
    except (ValueError,OSError,KeyError,subprocess.CalledProcessError) as e:
        print('Action pinning stopped:',e,file=sys.stderr);sys.exit(1)
