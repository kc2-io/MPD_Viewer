import { mkdtemp, mkdir, writeFile, readFile, rm, realpath, stat } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { randomBytes, createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { Desktop, sanitize } from './support.mjs';
import { demoSmoke, webSmoke, authSmoke, embeddedSmoke } from './specs.mjs';
import { VideoRecorder } from './video.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const results = [];
const xml = value => String(value).replace(/[\x00-\x08\x0b\x0c\x0e-\x1f]/g, '').replace(/[<>&"']/g, char => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' }[char]));
const binaryInput = process.env.MPD_E2E_BINARY;
if (!binaryInput || !path.isAbsolute(binaryInput)) throw new Error('MPD_E2E_BINARY must be an absolute path to an e2e-tests build');
const binary = await realpath(binaryInput);
async function binaryHash() { const hash = createHash('sha256'); for await (const chunk of createReadStream(binary)) hash.update(chunk); return hash.digest('hex'); }
const binarySha256 = await binaryHash();
if (!(await stat(binary)).isFile()) throw new Error('MPD_E2E_BINARY must be a file');
const output = path.resolve(process.env.MPD_E2E_OUTPUT_DIR || path.join(here, 'artifacts', `${Date.now()}`));
await mkdir(output, { recursive: true });
const root = await mkdtemp(path.join(os.tmpdir(), 'mpd-desktop-e2e-'));
const runId = randomBytes(16).toString('hex');
await writeFile(path.join(root, '.mpd-e2e-root'), runId, { flag: 'wx' });
const app = new Desktop({ binary, root, runId, output });
const recorder = new VideoRecorder(output);
const roots = [{ root, runId }];
async function activateRoot(name) {
  if (app.cancelled) throw new Error(app.cancelled);
  app.root = path.join(root, name); app.runId = randomBytes(16).toString('hex');
  roots.push({ root: app.root, runId: app.runId });
  await mkdir(app.root);
  await writeFile(path.join(app.root, '.mpd-e2e-root'), app.runId, { flag: 'wx' });
}
const started = Date.now();
const extended = process.env.MPD_E2E_EXTENDED === '1';
let active = 'initialization';
async function test(name, action) {
  if (app.cancelled) throw new Error(app.cancelled);
  active = name; const begin = Date.now();
  try { await action(); if (app.cancelled) throw new Error(app.cancelled); results.push({ name, seconds: (Date.now() - begin) / 1000 }); console.log(`PASS ${name}`); }
  catch (error) {
    results.push({ name, seconds: (Date.now() - begin) / 1000, failure: sanitize(error.stack).slice(0, 12000) });
    try { await app.screenshot(`failure-${results.length}`); } catch (captureError) { app.checkpoints.error = sanitize(captureError.message).slice(0, 256); }
    throw error;
  } finally {
    try { await app.checkpoint(name); } catch (captureError) { app.checkpoints.skipped++; app.checkpoints.error = sanitize(captureError.message).slice(0, 256); }
  }
}
let interrupt;
const interruption = new Promise((_, reject) => { interrupt = reason => { if (app.cancelled) return; app.cancelled = reason; reject(new Error(reason)); }; });
const onInterrupt = () => interrupt('Desktop suite interrupted by signal');
process.on('SIGINT', onInterrupt); process.on('SIGTERM', onInterrupt);
const deadline = setTimeout(() => interrupt('Desktop suite exceeded its overall deadline'), (extended ? 20 : 10) * 60 * 1000);
let suiteSettled = true;
try {
  await recorder.start();
  suiteSettled = false;
  const suite = (async () => {
    await demoSmoke(app, test, extended);
    await app.stop();
    await activateRoot('web');
    await webSmoke(app, test, extended);
    await app.stop();
    await activateRoot('embedded');
    await embeddedSmoke(app, test);
    await app.stop();
    await activateRoot('auth');
    await authSmoke(app, test);
    if (process.env.MPD_E2E_FORCE_FAILURE === '1') await test('intentional harness failure probe', () => { throw new Error('Requested failure to verify reports and cleanup'); });
    await app.stop();
    for (const scenario of ['demo', 'web']) {
      await activateRoot(`policy-${scenario}`);
      await test(`driverless ${scenario} native policy probe`, () => app.policyProbe(scenario));
      await app.stop();
    }
  })().finally(() => { suiteSettled = true; });
  try { await Promise.race([interruption, suite]); }
  catch (error) {
    // Give an in-flight command a short chance to settle, but preserve enough
    // workflow margin to finalize FFmpeg even if WebDriver is hung.
    if (app.cancelled) await Promise.race([
      suite.catch(() => {}),
      new Promise(resolve => setTimeout(resolve, 2000))
    ]);
    throw error;
  }
} catch (error) {
  if (!results.some(result => result.failure)) results.push({ name: active, seconds: 0, failure: sanitize(error.stack).slice(0, 12000) });
  console.error(sanitize(error.message)); process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  let visualEvidence = recorder.info;
  try { visualEvidence = await recorder.stop(Boolean(app.cancelled)); }
  catch (error) { visualEvidence.error = `Recorder finalization failed: ${sanitize(error.message).slice(0, 256)}`; }
  let cleanupComplete = true, rootRemoved = false;
  try {
    if (suiteSettled) await app.stop();
    else {
      await app.abortOwned();
      results.push({ name: 'interrupted suite cleanup', seconds: 0, failure: 'Retained isolated profile because an in-flight GUI action did not settle before recorder finalization' });
      cleanupComplete = false; process.exitCode = 1;
    }
  } catch (error) { results.push({ name: 'owned process cleanup', seconds: 0, failure: sanitize(error.message) }); cleanupComplete = false; process.exitCode = 1; }
  if (cleanupComplete) for (const ownedRoot of roots) {
    try { await app.cleanupRoot(ownedRoot.root, ownedRoot.runId); }
    catch (error) { results.push({ name: 'scoped credential cleanup', seconds: 0, failure: sanitize(error.message) }); cleanupComplete = false; process.exitCode = 1; break; }
  }
  // Never remove profiles/markers while an app or cleanup process may still hold them.
  if (cleanupComplete && !app.child && !app.cleanupChild) {
    try {
      if (path.dirname(root) !== os.tmpdir() || !path.basename(root).startsWith('mpd-desktop-e2e-') || await readFile(path.join(root, '.mpd-e2e-root'), 'utf8') !== runId) throw new Error('Refusing cleanup of unverified test root');
      await rm(root, { recursive: true }); rootRemoved = true;
    } catch (error) { results.push({ name: 'isolated profile cleanup', seconds: 0, failure: sanitize(error.message) }); cleanupComplete = false; process.exitCode = 1; }
  }
  if (!rootRemoved) console.error(`Retaining isolated test root after unresolved cleanup: ${root}`);
  try { await app.flushLogs(); } catch (error) { results.push({ name: 'application log drain', seconds: 0, failure: sanitize(error.message) }); process.exitCode = 1; }
  let commit = 'unknown'; try { commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: here, encoding: 'utf8' }).trim(); } catch {}
  let harnessDirty = null; try { harnessDirty = execFileSync('git', ['status', '--porcelain'], { cwd: here, encoding: 'utf8' }).trim().length > 0; } catch {}
  const binaryUnchanged = await binaryHash() === binarySha256;
  if (!binaryUnchanged) { results.push({ name: 'binary provenance', seconds: 0, failure: 'Binary changed during suite execution' }); process.exitCode = 1; }
  const manifest = {
    schema: 1, cleanup: { complete: cleanupComplete, rootRemoved, unresolvedOwnedProcess: Boolean(app.child || app.cleanupChild) }, harnessCommit: commit, harnessDirty, binarySha256, binaryUnchanged, buildCommit: /^[0-9a-f]{40}$/.test(process.env.MPD_E2E_BUILD_COMMIT || '') ? process.env.MPD_E2E_BUILD_COMMIT : null, platform: process.platform, architecture: process.arch, osRelease: os.release(), node: process.version,
    webdriverio: '9.32.0', driver: 'tauri-plugin-wdio-webdriver 1.4.0 (embedded W3C)',
    runnerImage: process.env.ImageOS || null, runnerImageVersion: process.env.ImageVersion || null,
    elapsedSeconds: (Date.now() - started) / 1000, extended, launches: app.launches, readiness: app.readiness, tests: results.map(({ name, failure }) => ({ name, result: failure ? 'failed' : 'passed' })),
    visualEvidence: { schema: 1, recorder: visualEvidence, checkpoints: app.checkpoints },
    evidenceBoundary: 'Real native Tauri windows and Rust backend with demo data. Embedded driver synthesizes DOM input; not OS input or Twitch acceptance.',
    notCovered: ['Native OS select/dropdown input (coverage uses labeled synthetic DOM events)', 'Native OS pointer/HTML5 drag', 'OS titlebar click (CloseRequested covered via native close API)', 'OS settings app theme toggle (window theme API covered)', 'Live Twitch authentication/playback/rewards', 'Real credential vault/account recovery (fake-token lifecycle covered)', 'Live Twitch remote-origin native IPC probe (driverless bundled/local origins covered)']
  };
  await writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
  await writeFile(path.join(output, 'results.xml'), `<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="MPD desktop GUI" tests="${results.length}" failures="${results.filter(result => result.failure).length}">${results.map(result => `<testcase name="${xml(result.name)}" time="${result.seconds}">${result.failure ? `<failure>${xml(result.failure)}</failure>` : ''}</testcase>`).join('')}</testsuite></testsuites>`);
  process.removeListener('SIGINT', onInterrupt); process.removeListener('SIGTERM', onInterrupt);
  console.log(`Evidence: ${output}`);
}
