#!/usr/bin/env python3
"""Inspect reference repos, prepare reproducibility, or create PRIVATE OWNER/MPD_Viewer.
Requires authenticated GitHub CLI. Does not copy secrets, configure Azure/Apple,
enable releases, create tags, deploy the website, or modify the reference repos.
"""
from __future__ import annotations
import argparse
import base64
import json
from pathlib import Path
import re
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
REFERENCES=('BotOrNot','mpd-bot')
ENVIRONMENTS=('release-windows','release-macos','release-publish')
CHECKS=('Source checks','Native (Windows-x64)','Native (macOS-arm64)','Native (macOS-x64)','Native (Linux-x64)')

def run(*args, capture=False):
    p=subprocess.run(args,cwd=ROOT,check=True,text=True,stdout=subprocess.PIPE if capture else None)
    return p.stdout.strip() if capture else ''

def api(path, method='GET', body=None):
    args=['gh','api',path,'--method',method]
    if body is not None:args+=['--input','-']
    p=subprocess.run(args,cwd=ROOT,input=json.dumps(body) if body is not None else None,
                     text=True,check=True,stdout=subprocess.PIPE)
    return json.loads(p.stdout) if p.stdout.strip() else None

def audit(owner):
    dest=ROOT/'.audit-local';dest.mkdir(exist_ok=True)
    result={}
    for repo in REFERENCES:
        full=owner+'/'+repo
        metadata=api('repos/'+full)
        files=api('repos/'+full+'/contents/.github/workflows')
        target=dest/repo;target.mkdir(exist_ok=True)
        names=[]
        for item in files:
            name=item['name']
            if item.get('type')!='file' or not name.endswith(('.yml','.yaml')):continue
            if '/' in name or '\\' in name or name.startswith('.'):raise ValueError('Unexpected workflow path.')
            data=api('repos/'+full+'/contents/'+item['path'])
            if data.get('encoding')!='base64':raise ValueError('Unexpected workflow encoding.')
            (target/name).write_bytes(base64.b64decode(data['content']))
            names.append({'name':name,'git_blob_sha':data['sha']})
        # Secret values are not readable from GitHub; do not attempt to extract them.
        result[full]={'default_branch':metadata['default_branch'],'private':metadata['private'],'workflows':names}
    (dest/'references.json').write_text(json.dumps(result,indent=2)+'\n', encoding="utf-8")
    print('Reference workflows downloaded read-only to .audit-local/. Compare signing/environment names before enabling releases.')

def prepare():
    if (ROOT/'Cargo.lock').exists():
        # Preparing repository settings must not unexpectedly update existing dependencies.
        run('cargo','metadata','--locked','--format-version','1',capture=True)
    else:
        run('cargo','generate-lockfile')
    run(sys.executable,'scripts/release-tools.py','pin-rust')
    run(sys.executable,'scripts/pin-actions.py','--write')
    print('Review and commit Cargo.lock, rust-toolchain.toml, .github/action-pins.json and workflow changes.')

def create(owner):
    full=owner+'/MPD_Viewer'
    if run('git','branch','--show-current',capture=True)!='main':raise ValueError('Use the main branch for initial creation.')
    if run('git','status','--porcelain',capture=True):raise ValueError('Commit or remove local changes first.')
    if 'origin' in run('git','remote',capture=True).splitlines():raise ValueError('An origin remote already exists; refusing to replace it.')
    actor=api('user')
    audit(owner) # Any missing reference access fails before creating a repository.
    run('gh','repo','create',full,'--private','--description','MPD Viewer — view favorite Twitch channels in priority','--disable-wiki')
    print('Created PRIVATE',full,'; applying settings. On failure this private repository is left in place for inspection.')
    settings=api('repos/'+full)
    if settings.get('private') is not True:raise ValueError('Privacy verification failed; refusing to push source.')
    api('repos/'+full,'PATCH',{'has_issues':True,'has_wiki':False,'has_projects':False,
        'allow_squash_merge':True,'allow_merge_commit':False,'allow_rebase_merge':False,
        'delete_branch_on_merge':True,'allow_auto_merge':False})
    api('repos/'+full+'/actions/permissions/workflow','PUT',{'default_workflow_permissions':'read','can_approve_pull_request_reviews':False})
    for name in ('RELEASES_ENABLED','STABLE_RELEASES_ENABLED'):
        api('repos/'+full+'/actions/variables','POST',{'name':name,'value':'false'})
    # No signing material exists here and release flags are false; preserve useful source even if a later plan-specific protection fails.
    run('git','remote','add','origin',settings['clone_url'])
    run('git','-c','credential.helper=!gh auth git-credential','push','--set-upstream','origin','main')
    api('repos/'+full,'PATCH',{'default_branch':'main'})
    for name in ENVIRONMENTS:
        # Plans that cannot enforce these protections must fail closed; never silently drop approval requirements.
        api('repos/'+full+'/environments/'+name,'PUT',{
            'reviewers':[{'type':'User','id':actor['id']}],
            'prevent_self_review':False,
            'deployment_branch_policy':{'protected_branches':False,'custom_branch_policies':True}})
        api('repos/'+full+'/environments/'+name+'/deployment-branch-policies','POST',{'name':'v*','type':'tag'})
    api('repos/'+full+'/branches/main/protection','PUT',{
        'required_status_checks':{'strict':True,'contexts':list(CHECKS)},'enforce_admins':True,
        'required_pull_request_reviews':{'dismiss_stale_reviews':True,'require_code_owner_reviews':False,'required_approving_review_count':0},
        'restrictions':None,'required_linear_history':True,'allow_force_pushes':False,
        'allow_deletions':False,'required_conversation_resolution':True})
    api('repos/'+full+'/rulesets','POST',{
        'name':'Immutable release tags','target':'tag','enforcement':'active',
        'conditions':{'ref_name':{'include':['refs/tags/v*'],'exclude':[]}},
        'rules':[{'type':'update'},{'type':'deletion'}]})
    if api('repos/'+full).get('private') is not True:raise ValueError('Final privacy verification failed.')
    print('Source pushed and protections configured. Releases remain disabled; signing credentials and OIDC trust are NOT configured.')

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('operation',choices=['inspect','prepare','create']);p.add_argument('--owner',default='kc2-io')
    a=p.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9-]{0,38}',a.owner):raise ValueError('Invalid GitHub owner.')
    run('gh','auth','status')
    if a.operation=='inspect':audit(a.owner)
    elif a.operation=='prepare':prepare()
    else:create(a.owner)
if __name__=='__main__':
    try:main()
    except (ValueError,OSError,KeyError,subprocess.CalledProcessError) as e:
        print('Setup stopped:',e,file=sys.stderr)
        print('No secret values were copied. Inspect partial remote state before retrying create; no repository is deleted or overwritten.',file=sys.stderr)
        sys.exit(1)
