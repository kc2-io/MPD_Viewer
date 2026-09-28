import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
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
