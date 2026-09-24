import { mkdtemp, mkdir, writeFile, readFile, rm, realpath, stat } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { Desktop, sanitize } from './support.mjs';
import { demoSmoke } from './specs.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const results = [];
const xml = value => String(value).replace(/[<>&"']/g, char => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' }[char]));
const binaryInput = process.env.MPD_E2E_BINARY;
if (!binaryInput || !path.isAbsolute(binaryInput)) throw new Error('MPD_E2E_BINARY must be an absolute path to an e2e-tests build');
const binary = await realpath(binaryInput);
if (!(await stat(binary)).isFile()) throw new Error('MPD_E2E_BINARY must be a file');
const output = path.resolve(process.env.MPD_E2E_OUTPUT_DIR || path.join(here, 'artifacts', `${Date.now()}`));
await mkdir(output, { recursive: true });
const root = await mkdtemp(path.join(os.tmpdir(), 'mpd-desktop-e2e-'));
const runId = randomBytes(16).toString('hex');
await writeFile(path.join(root, '.mpd-e2e-root'), runId, { flag: 'wx' });
const app = new Desktop({ binary, root, runId, output });
const started = Date.now();
const extended = process.env.MPD_E2E_EXTENDED === '1';
let active = 'initialization';
async function test(name, action) {
  active = name; const begin = Date.now();
  try { await action(); results.push({ name, seconds: (Date.now() - begin) / 1000 }); console.log(`PASS ${name}`); }
  catch (error) {
    results.push({ name, seconds: (Date.now() - begin) / 1000, failure: sanitize(error.stack).slice(0, 12000) });
    await app.screenshot(`failure-${results.length}`); throw error;
  }
}
const deadline = setTimeout(() => {
  console.error('Desktop suite exceeded its overall deadline'); process.exitCode = 1;
  void app.stop();
}, (extended ? 20 : 10) * 60 * 1000);
try {
  await demoSmoke(app, test, extended);
  if (process.env.MPD_E2E_FORCE_FAILURE === '1') await test('intentional harness failure probe', () => { throw new Error('Requested failure to verify reports and cleanup'); });
} catch (error) {
  if (!results.some(result => result.failure)) results.push({ name: active, seconds: 0, failure: sanitize(error.stack).slice(0, 12000) });
  console.error(sanitize(error.message)); process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  try { await app.stop(); } catch (error) { results.push({ name: 'owned process cleanup', seconds: 0, failure: sanitize(error.message) }); process.exitCode = 1; }
  let commit = 'unknown'; try { commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: here, encoding: 'utf8' }).trim(); } catch {}
  const manifest = {
    schema: 1, commit, platform: process.platform, architecture: process.arch, osRelease: os.release(), node: process.version,
    webdriverio: '9.30.1', driver: 'tauri-plugin-wdio-webdriver 1.4.0 (embedded W3C)',
    runnerImage: process.env.ImageOS || null, runnerImageVersion: process.env.ImageVersion || null,
    elapsedSeconds: (Date.now() - started) / 1000, extended, launches: app.launches, tests: results.map(({ name, failure }) => ({ name, result: failure ? 'failed' : 'passed' })),
    evidenceBoundary: 'Real native Tauri windows and Rust backend with demo data. Embedded driver synthesizes DOM input; not OS input or Twitch acceptance.',
    notCovered: ['Native OS pointer/HTML5 drag', 'Native CloseRequested event', 'OS theme toggle', 'Live Twitch authentication/playback/rewards', 'Native credential vault recovery', 'Full-page fixture lifecycle', 'Unprivileged native IPC probes']
  };
  await writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
  await writeFile(path.join(output, 'results.xml'), `<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="MPD desktop GUI" tests="${results.length}" failures="${results.filter(result => result.failure).length}">${results.map(result => `<testcase name="${xml(result.name)}" time="${result.seconds}">${result.failure ? `<failure>${xml(result.failure)}</failure>` : ''}</testcase>`).join('')}</testsuite></testsuites>`);
  // Remove only a root created by this process and still bearing its exact ownership marker.
  if (path.dirname(root) === os.tmpdir() && path.basename(root).startsWith('mpd-desktop-e2e-') && await readFile(path.join(root, '.mpd-e2e-root'), 'utf8') === runId) await rm(root, { recursive: true });
  else { console.error('Refusing cleanup of unverified test root'); process.exitCode = 1; }
  console.log(`Evidence: ${output}`);
}
