import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { appendFile, mkdir, writeFile } from 'node:fs/promises';
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
    .replace(/((?:access_token|refresh_token|authorization|password|device_code|user_code)\s*[=:]\s*)[^\s,}]+/gi, '$1[redacted]');
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
    if (this.child) throw new Error('Previous owned app must be stopped before launch');
    const port = await freePort();
    const logFile = path.join(this.output, `app-${this.launches.length + 1}.log`);
    let logBytes = 0;
    const child = spawn(this.binary, args, { shell: false, windowsHide: false, detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'], env: environment({
      MPD_E2E_ROOT: this.root, MPD_E2E_RUN_ID: this.runId, MPD_E2E_SCENARIO: scenario,
      TAURI_WEBDRIVER_PORT: String(port), WDIO_EMBEDDED_SERVER: 'true', RUST_LOG: 'warn'
    }) });
    this.child = child;
    this.launches.push({ pid: child.pid, scenario, args, port });
    for (const stream of [child.stdout, child.stderr]) stream.on('data', data => {
      const text = sanitize(data.toString()); const available = Math.max(0, 256 * 1024 - logBytes);
      logBytes += Buffer.byteLength(text); if (available) void appendFile(logFile, text.slice(0, available)).catch(() => {});
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
    await until(async () => (await this.browser.$('#run-status')).isExisting(), 'Manager did not render');
    this.manager = await this.browser.getWindowHandle();
    await this.browser.setWindowRect(0, 0, 1400, 1000);
    return this.browser;
  }
  async screenshot(name) {
    if (!this.browser) return;
    let handles;
    try { handles = await this.browser.getWindowHandles(); } catch { return; }
    for (const [index, handle] of handles.slice(0, 8).entries()) {
      try {
        await this.browser.switchToWindow(handle);
        const base64 = await this.browser.takeScreenshot();
        const buffer = Buffer.from(base64, 'base64');
        if (buffer.length <= 8 * 1024 * 1024) await writeFile(path.join(this.output, `${name}-${index}.png`), buffer);
      } catch (error) { await appendFile(path.join(this.output, 'capture.log'), `${sanitize(error.message).slice(0, 1000)}\n`); }
    }
    try { await this.browser.switchToWindow(this.manager); } catch {}
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
export async function order(browser) { return Promise.all((await browser.$$('#favorites > li')).map(row => row.getAttribute('data-login'))); }
export async function windows(browser, count) { return until(async () => { const handles = await browser.getWindowHandles(); return handles.length === count && handles; }, `Expected ${count} actual native window handles`); }
