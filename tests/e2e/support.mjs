import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { appendFile, mkdir, writeFile, rename, readFile } from 'node:fs/promises';
import path from 'node:path';
import { remote } from 'webdriverio';

export const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
export async function until(check, message, timeout = 15000) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    try { const result = await check(); if (result) return result; } catch (error) { last = error; }
    await delay(150);
  }
  throw new Error(`${message}${last ? `: ${last.message}` : ''}`);
}
export function sanitize(text) {
  return String(text).replace(/https?:\/\/[^\s"<>]+/g, value => value.replace(/[?#].*/, '?[redacted]'))
    .replace(/\bBearer\s+[^\s,}"']+/gi, 'Bearer [redacted]')
    .replace(/(["']?(?:access_token|refresh_token|authorization|password|device_code|user_code)["']?\s*[=:]\s*)(?:"[^"]*"|'[^']*'|[^\s,}]+)/gi, '$1[redacted]');
}
function environment(extra) {
  const allowed = /^(PATH|PATHEXT|SYSTEMROOT|WINDIR|COMSPEC|TEMP|TMP|TMPDIR|HOME|USERPROFILE|APPDATA|LOCALAPPDATA|PROGRAMFILES.*|COMMONPROGRAMFILES.*|DISPLAY|WAYLAND_DISPLAY|XAUTHORITY|DBUS_SESSION_BUS_ADDRESS|XDG_RUNTIME_DIR|LANG|LC_.*|LD_LIBRARY_PATH|DYLD_.*|WEBVIEW2_BROWSER_EXECUTABLE_FOLDER)$/i;
  return { ...Object.fromEntries(Object.entries(process.env).filter(([key]) => allowed.test(key))), ...extra };
}
export async function freePort() {
  const server = createServer();
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const port = server.address().port; await new Promise(resolve => server.close(resolve)); return port;
}
export class Desktop {
  constructor(options) { Object.assign(this, options); this.launches = []; }
  async start(scenario = 'demo', args = []) {
    if (this.cancelled) throw new Error(this.cancelled);
    if (this.child) throw new Error('Previous owned app must be stopped before launch');
    this.spawnError = null;
    const port = await freePort();
    const logFile = path.join(this.output, `app-${this.launches.length + 1}.log`);
    let logBytes = 0;
    await writeFile(logFile, '');
    const child = spawn(this.binary, args, { shell: false, windowsHide: false, detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'], env: environment({
      MPD_E2E_ROOT: this.root, MPD_E2E_RUN_ID: this.runId, MPD_E2E_SCENARIO: scenario,
      TAURI_WEBDRIVER_PORT: String(port), WDIO_EMBEDDED_SERVER: 'true', RUST_LOG: 'warn'
    }) });
    this.child = child;
    this.launches.push({ pid: child.pid, scenario, args, port });
    for (const stream of [child.stdout, child.stderr]) stream.on('data', data => {
      const text = sanitize(data.toString()); const available = Math.max(0, 256 * 1024 - logBytes);
      logBytes += Buffer.byteLength(text); if (available) void appendFile(logFile, Buffer.from(text).subarray(0, available)).catch(() => {});
    });
    child.on('error', error => { this.spawnError = error; });
    await until(async () => {
      if (this.spawnError) throw this.spawnError;
      if (child.exitCode !== null) throw new Error(`Test app exited (${child.exitCode})`);
      const response = await fetch(`http://127.0.0.1:${port}/status`, { signal: AbortSignal.timeout(1000) });
      return response.ok && (await response.json()).value?.ready;
    }, 'Embedded driver not ready', 45000);
    this.browser = await remote({ hostname: '127.0.0.1', port, path: '/', logLevel: 'silent', connectionRetryCount: 0,
      connectionRetryTimeout: 15000, capabilities: { browserName: 'wry', 'tauri:options': { application: this.binary } } });
    await until(() => this.browser.execute(() => { const backend = document.querySelector('#viewer-backend')?.textContent?.trim(); return Boolean(backend && backend !== 'unknown'); }), 'Manager did not render its first Rust state');
    this.manager = await this.browser.getWindowHandle();
    await this.browser.setWindowRect(0, 0, 1400, 1000);
    this.launches.at(-1).renderer = await this.browser.execute(() => ({ userAgent: navigator.userAgent, width: innerWidth, height: innerHeight, scale: devicePixelRatio }));
    return this.browser;
  }
  async nativeCommand(command) {
    const id = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const temporary = path.join(this.root, `command-${id}.tmp`);
    await writeFile(temporary, JSON.stringify({ ...command, id }));
    await rename(temporary, path.join(this.root, 'native-command.json'));
    return until(async () => {
      const result = JSON.parse(await readFile(path.join(this.root, 'native-command-result.json'), 'utf8'));
      if (result.id !== id) return false;
      if (!result.ok) throw new Error(`Native ${command.action} failed: ${result.error}`);
      return true;
    }, `Native ${command.action} acknowledgement missing`);
  }
  async screenshot(name) {
    if (!this.browser) return;
    let handles;
    try { handles = await this.browser.getWindowHandles(); } catch { return; }
    let captured = 0;
    for (const [index, handle] of handles.slice(0, 8).entries()) {
      try {
        await this.browser.switchToWindow(handle);
        const base64 = await this.browser.takeScreenshot();
        const buffer = Buffer.from(base64, 'base64');
        if (buffer.length > 8 * 1024 * 1024 || buffer.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('Screenshot must be a bounded PNG');
        await writeFile(path.join(this.output, `${name}-${index}.png`), buffer);
        captured++;
      } catch (error) { await appendFile(path.join(this.output, 'capture.log'), `${sanitize(error.message).slice(0, 1000)}\n`); }
    }
    try { await this.browser.switchToWindow(this.manager); } catch {}
    return captured;
  }
  async policyProbe(scenario) {
    if (this.cancelled) throw new Error(this.cancelled);
    if (this.child) throw new Error('Stop the owned GUI before a driverless policy probe');
    const port = await freePort();
    const child = spawn(this.binary, [], { shell: false, windowsHide: false, detached: process.platform !== 'win32', stdio: 'ignore', env: environment({ MPD_E2E_ROOT: this.root, MPD_E2E_RUN_ID: this.runId, MPD_E2E_SCENARIO: scenario, MPD_E2E_POLICY_PROBE: '1', TAURI_WEBDRIVER_PORT: String(port) }) });
    this.child = child;
    this.launches.push({ pid: child.pid, scenario, policyProbe: true, port });
    let error; child.on('error', value => { error = value; });
    await until(() => { if (error) throw error; return child.exitCode !== null; }, 'Driverless native policy probe did not exit', 60000);
    const report = await readFile(path.join(this.root, 'policy-probe.json'), 'utf8');
    if (Buffer.byteLength(report) > 64 * 1024) throw new Error('Native policy report exceeds artifact limit');
    const parsed = JSON.parse(report);
    await writeFile(path.join(this.output, `policy-${scenario}.json`), sanitize(JSON.stringify(parsed, null, 2)));
    if (parsed.passed !== true || parsed.driver_registered !== false || parsed.scenario !== scenario || !Array.isArray(parsed.results) || parsed.results.length < 2) throw new Error('Driverless policy report did not establish expected native coverage');
    const expected = scenario === 'demo' ? ['dispatch_denied', 'get_state_denied', 'own_report_allowed', 'wrong_session_denied'] : ['dispatch_denied', 'get_state_denied', 'player_report_denied'];
    if (parsed.results.some(result => expected.some(key => result.checks?.[key] !== true))) throw new Error('Driverless policy assertion failed');
    if (child.exitCode !== 0) throw new Error(`Driverless native policy probe exited ${child.exitCode}; see policy-${scenario}.json`);
  }
  async fixtureState(state) {
    const temporary = path.join(this.root, 'fixture-state.tmp');
    await writeFile(temporary, JSON.stringify(state));
    await rename(temporary, path.join(this.root, 'fixture-state.json'));
  }
  async cleanupRoot(root, runId) {
    const port = await freePort();
    const child = spawn(this.binary, ['--e2e-cleanup'], { shell: false, windowsHide: true, stdio: 'ignore', env: environment({ MPD_E2E_ROOT: root, MPD_E2E_RUN_ID: runId, TAURI_WEBDRIVER_PORT: String(port) }) });
    let error; child.on('error', value => { error = value; });
    try {
      await until(() => { if (error) throw error; return child.exitCode !== null; }, 'Scoped credential cleanup did not exit', 15000);
      if (child.exitCode !== 0) throw new Error(`Scoped credential cleanup exited ${child.exitCode}`);
    } finally { if (child.exitCode === null && child.signalCode === null) child.kill(); }
  }
  async stop() {
    try { await this.browser?.deleteSession(); } catch {}
    this.browser = null;
    const child = this.child; this.child = null;
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    // Only this exact child, and its process tree/group, may be terminated. Never kill by executable name.
    if (process.platform === 'win32') spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, timeout: 10000, stdio: 'ignore' });
    else { try { process.kill(-child.pid, 'SIGTERM'); } catch {} }
    await until(() => child.exitCode !== null || child.signalCode !== null, 'Owned app did not terminate', 10000);
  }
}
export async function visibleText(browser, selector) { return (await browser.$(selector)).getText(); }
export async function click(browser, selector) { await (await browser.$(selector)).click(); }
export async function input(browser, selector, value) { const element = await browser.$(selector); await element.setValue(String(value)); }
export const byLabel = label => `[aria-label="${label}"]`;
export async function order(browser) { return browser.execute(() => [...document.querySelectorAll('#favorites > li')].map(row => row.dataset.login)); }
export async function windows(browser, count) { return until(async () => { const handles = await browser.getWindowHandles(); return handles.length === count && handles; }, `Expected ${count} actual native window handles`); }


// The embedded driver cannot select OPTIONs or start HTML5 drags. These narrowly
// scoped DOM input helpers exercise the real UI handlers, never native dispatch.
// They are synthetic input coverage, not OS pointer/keyboard acceptance.
export async function selectValue(browser, selector, value) {
  await browser.execute((selector, value) => {
    const select = document.querySelector(selector);
    if (!(select instanceof HTMLSelectElement) || ![...select.options].some(option => option.value === value)) throw new Error('Expected select and option');
    select.focus(); select.value = value;
    select.dispatchEvent(new Event('input', { bubbles: true }));
    select.dispatchEvent(new Event('change', { bubbles: true }));
    select.blur();
  }, selector, value);
}
export async function dragBefore(browser, sourceLogin, targetLogin) {
  await browser.execute((sourceLogin, targetLogin) => {
    const rows = [...document.querySelectorAll('#favorites > li')];
    const source = rows.find(row => row.dataset.login === sourceLogin);
    const target = rows.find(row => row.dataset.login === targetLogin);
    if (!source || !target) throw new Error('Drag source/target missing');
    const transfer = new DataTransfer();
    const rect = target.getBoundingClientRect();
    source.dispatchEvent(new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: transfer }));
    const options = { bubbles: true, cancelable: true, dataTransfer: transfer, clientX: rect.left + rect.width / 2, clientY: rect.top + 1 };
    target.dispatchEvent(new DragEvent('dragover', options));
    target.dispatchEvent(new DragEvent('drop', options));
    source.dispatchEvent(new DragEvent('dragend', { bubbles: true, dataTransfer: transfer }));
  }, sourceLogin, targetLogin);
}
