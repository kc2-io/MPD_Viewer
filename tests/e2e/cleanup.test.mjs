import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, rm, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { isolatedRootRemoveOptions, removeIsolatedRoot } from './cleanup.mjs';

async function fixture(t, marker = 'a'.repeat(32)) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'mpd-desktop-e2e-'));
  await writeFile(path.join(root, '.mpd-e2e-root'), marker, { flag: 'wx' });
  t.after(() => rm(root, { recursive: true, force: true }));
  return { root, marker };
}

test('isolated profile cleanup configures bounded transient-lock retries', async t => {
  const { root, marker } = await fixture(t);
  let observed;
  await removeIsolatedRoot(root, marker, async (target, options) => { observed = { target, options }; });
  assert.equal(observed.target, root);
  assert.deepEqual(observed.options, {
    recursive: true,
    force: true,
    maxRetries: 8,
    retryDelay: 250
  });
  assert.equal(Object.isFrozen(isolatedRootRemoveOptions), true);
});

test('isolated profile cleanup waits for a transient Windows lock', { skip: process.platform !== 'win32', timeout: 15000 }, async t => {
  const { root, marker } = await fixture(t);
  const locked = path.join(root, 'webview-lockfile');
  await writeFile(locked, 'locked');
  const script = [
    '$stream = [System.IO.File]::Open($env:MPD_E2E_LOCK_PATH, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::None);',
    'try { [Console]::Out.WriteLine("locked"); [Console]::Out.Flush(); Start-Sleep -Milliseconds 900 }',
    'finally { $stream.Dispose() }'
  ].join(' ');
  const child = spawn('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], {
    env: { ...process.env, MPD_E2E_LOCK_PATH: locked },
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe']
  });
  const exited = new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => code === 0 ? resolve() : reject(new Error(`Lock holder exited ${code ?? signal}`)));
  });
  const ready = new Promise((resolve, reject) => {
    child.once('error', reject);
    child.stdout.once('data', chunk => chunk.toString().includes('locked') ? resolve() : reject(new Error('Lock holder was not ready')));
  });
  await Promise.race([ready, exited.then(() => { throw new Error('Lock holder exited before acquiring the lock'); })]);
  const started = Date.now();
  await removeIsolatedRoot(root, marker);
  assert.ok(Date.now() - started >= 500);
  await exited;
  await assert.rejects(stat(root), { code: 'ENOENT' });
});

test('isolated profile cleanup still reports a persistent lock failure', async t => {
  const { root, marker } = await fixture(t);
  const busy = Object.assign(new Error('resource busy or locked'), { code: 'EBUSY' });
  await assert.rejects(removeIsolatedRoot(root, marker, async () => { throw busy; }), error => error === busy);
});

test('isolated profile cleanup refuses an unverified root', async t => {
  const { root } = await fixture(t, 'expected-marker');
  let called = false;
  await assert.rejects(
    removeIsolatedRoot(root, 'different-marker', async () => { called = true; }),
    /Refusing cleanup of unverified test root/
  );
  assert.equal(called, false);
});
