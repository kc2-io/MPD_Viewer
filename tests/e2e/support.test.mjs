import test, { mock } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, chmod, readdir, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { Desktop, isRetryablePolicyObservation } from './support.mjs';

test('only native observation timeouts can receive one startup retry', () => {
  const base = { passed: false, driver_registered: false, results: null };
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Timed out waiting for two native IPC probe results; viewers=2; results=1; url_read_failures=3' }, 1), true);
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Driverless policy probe timed out' }, 1), true);
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Native IPC policy check failed for player-1' }, 1), false);
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Timed out waiting for two native IPC probe results; viewers=2', results: [] }, 1), false);
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Driverless policy probe timed out', driver_registered: true }, 1), false);
  assert.equal(isRetryablePolicyObservation({ ...base, error: 'Driverless policy probe timed out' }, 0), false);
});

test('driverless launcher retains the timed-out attempt and validates the retry', { skip: process.platform === 'win32' }, async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'mpd-policy-launcher-test-'));
  const binary = path.join(root, 'fake-app');
  const output = path.join(root, 'output');
  await mkdir(output);
  await writeFile(binary, `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const root = process.env.MPD_E2E_ROOT;
const marker = path.join(root, 'fake-attempt');
const attempt = fs.existsSync(marker) ? 2 : 1;
fs.writeFileSync(marker, String(attempt));
const report = attempt === 1
  ? { passed: false, driver_registered: false, scenario: 'web', results: null,
      error: 'Timed out waiting for two native IPC probe results; viewers=1; results=0; url_read_failures=2' }
  : { passed: true, driver_registered: false, scenario: 'web', error: null,
      results: [{ checks: { dispatch_denied: true, get_state_denied: true, player_report_denied: true } },
                { checks: { dispatch_denied: true, get_state_denied: true, player_report_denied: true } }] };
fs.writeFileSync(path.join(root, 'policy-probe.json'), JSON.stringify(report));
process.stderr.write('fake probe attempt ' + attempt + '\\n');
process.exit(attempt === 1 ? 1 : 0);
`);
  await chmod(binary, 0o755);
  const desktop = new Desktop({ binary, root, output, runId: 'a'.repeat(32) });
  const warnings = [];
  const log = mock.method(console, 'log', message => warnings.push(message));
  try {
    await desktop.policyProbe('web');
    assert.equal(desktop.launches.length, 2);
    assert.equal(desktop.launches[0].retryReason, 'fixture_observation_timeout');
    assert.equal(desktop.launches[1].retryReason, undefined);
    assert.deepEqual(warnings, ['::warning title=Driverless policy probe retry::web fixture observation timed out; retrying once. See policy-web-attempt-1.json']);
    const first = JSON.parse(await readFile(path.join(output, 'policy-web-attempt-1.json'), 'utf8'));
    const second = JSON.parse(await readFile(path.join(output, 'policy-web-attempt-2.json'), 'utf8'));
    assert.equal(first.passed, false);
    assert.equal(second.passed, true);
    assert.equal((await readdir(output)).filter(name => name.endsWith('.log')).length, 2);
  } finally {
    log.mock.restore();
    await desktop.stop();
    await rm(root, { recursive: true });
  }
});
