import { readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

// Recursive rm retries these transient Windows lock errors with linear backoff.
// Eight 250 ms increments cap the additional wait at nine seconds.
export const isolatedRootRemoveOptions = Object.freeze({
  recursive: true,
  force: true,
  maxRetries: 8,
  retryDelay: 250
});

export async function removeIsolatedRoot(root, runId, remove = rm) {
  const marker = path.join(root, '.mpd-e2e-root');
  if (path.dirname(root) !== os.tmpdir() ||
      !path.basename(root).startsWith('mpd-desktop-e2e-') ||
      await readFile(marker, 'utf8') !== runId) {
    throw new Error('Refusing cleanup of unverified test root');
  }
  await remove(root, isolatedRootRemoveOptions);
}
