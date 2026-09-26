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
export async function documentTitle(browser, expected) {
  // Driver 1.4.0 getTitle is one document.title evaluation. execute/sync instead
  // stores a result in a window-global then polls it; initial navigation can
  // destroy that global. Wait for the final document before any execute call.
  await until(async () => await browser.getTitle() === expected, `Expected final document title: ${expected}`);
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
function exited(child) { return child.exitCode !== null || child.signalCode !== null; }
async function terminateOwned(child) {
  if (exited(child)) return;
  // The process object remains owned until exit is confirmed. Never target an executable name.
  if (process.platform === 'win32') {
    spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, timeout: 10000, stdio: 'ignore' });
    await until(() => exited(child), 'Owned Windows process tree did not terminate', 10000);
    return;
  }
  try { process.kill(-child.pid, 'SIGTERM'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  try { await until(() => exited(child), 'Owned process ignored SIGTERM', 10000); }
  catch {
    // Only this still-owned process group is escalated; retain its handle if SIGKILL also fails.
    if (!exited(child)) {
      try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      await until(() => exited(child), 'Owned process group did not terminate after SIGKILL', 10000);
    }
  }
}
export class Desktop {
  constructor(options) { Object.assign(this, options); this.launches = []; this.readiness = []; this.logWrites = Promise.resolve(); this.logErrors = []; this.checkpoints = { enabled: process.env.MPD_E2E_CHECKPOINTS !== '0', attempted: 0, captured: 0, skipped: 0, quota: 64, cases: [] }; }
  async start(scenario = 'demo', args = []) {
    if (this.cancelled) throw new Error(this.cancelled);
    if (this.child) throw new Error('Previous owned app must be stopped before launch');
    this.spawnError = null;
    const port = await freePort();
    if (this.cancelled) throw new Error(this.cancelled);
    const logFile = path.join(this.output, `app-${this.launches.length + 1}.log`);
    let logBytes = 0;
    await writeFile(logFile, '');
    const child = spawn(this.binary, args, { shell: false, windowsHide: false, detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'], env: environment({
      MPD_E2E_ROOT: this.root, MPD_E2E_RUN_ID: this.runId, MPD_E2E_SCENARIO: scenario,
      TAURI_WEBDRIVER_PORT: String(port), WDIO_EMBEDDED_SERVER: 'true', RUST_LOG: 'warn'
    }) });
    this.child = child;
    this.logClosed = new Promise(resolve => child.once('close', resolve));
    this.launches.push({ pid: child.pid, scenario, args, port });
    for (const stream of [child.stdout, child.stderr]) stream.on('data', data => {
      const text = sanitize(data.toString()); const available = Math.max(0, 256 * 1024 - logBytes);
      logBytes += Buffer.byteLength(text);
      if (available) {
        const chunk = Buffer.from(text).subarray(0, available);
        // Preserve arrival order across both streams and finish writes before evidence.
        this.logWrites = this.logWrites.then(() => appendFile(logFile, chunk)).catch(error => {
          if (this.logErrors.length < 4) this.logErrors.push(sanitize(error.message).slice(0, 512));
        });
      }
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
    await documentTitle(this.browser, 'MPD Viewer');
    await until(() => this.browser.execute(() => { const backend = document.querySelector('#viewer-backend')?.textContent?.trim(); const origin = document.querySelector('#player-origin')?.textContent?.trim(); return Boolean(backend && backend !== 'unknown' && origin); }), 'Manager did not render its first Rust state');
    this.manager = await this.browser.getWindowHandle();
    // Driver 1.4.0 setWindowRect uses one-shot event callbacks that can panic
    // when Linux replays resize events. Use the narrow native API mailbox.
    const observeGeometry = () => this.browser.execute(() => ({ width: innerWidth, height: innerHeight, scale: devicePixelRatio,
      screenWidth: screen.width, screenHeight: screen.height, availableWidth: screen.availWidth, availableHeight: screen.availHeight }));
    const geometry = { requested: { width: 1000, height: 650 }, before: await observeGeometry(), confirmed: false, widthChanged: false, last: null };
    this.launches.at(-1).geometry = geometry;
    if (!Number.isFinite(geometry.before?.width)) throw new Error('Manager initial viewport width is not finite');
    await this.nativeCommand({ action: 'resize', label: this.manager, ...geometry.requested });
    try {
      await until(async () => {
        geometry.last = await observeGeometry();
        const view = geometry.last;
        geometry.widthChanged = Number.isFinite(view?.width) && view.width !== geometry.before.width;
        // macOS frame/content sizing can reduce CSS height by its titlebar. Keep
        // horizontal resizing exact and require a bounded usable content area.
        geometry.confirmed = Number.isFinite(view?.width) && Number.isFinite(view?.height) && Number.isFinite(view?.availableHeight) &&
          view.width === geometry.requested.width && view.height >= 600 && view.height <= Math.min(650, view.availableHeight) &&
          (geometry.before.width === geometry.requested.width || geometry.widthChanged);
        return geometry.confirmed;
      }, 'Native horizontal resize did not produce the requested width and usable viewport');
    } catch (error) { throw new Error(`${error.message}; observed geometry: ${JSON.stringify(geometry)}`); }
    this.launches.at(-1).renderer = { ...geometry.last, userAgent: await this.browser.execute(() => navigator.userAgent) };
    return this.browser;
  }
  async viewerReady({ title, channels, demo = false }) {
    await documentTitle(this.browser, title);
    const observation = { label: await this.browser.getWindowHandle(), expectedTitle: title, expectedChannels: channels, passed: false, last: null };
    if (this.readiness.length < 16) this.readiness.push(observation);
    const sample = async () => {
        const raw = await this.browser.execute(demo => {
          const text = selector => document.querySelector(selector)?.textContent?.trim().slice(0, 160) ?? null;
          return { readyState: document.readyState, title: document.title.slice(0, 160),
            channel: text(demo ? '#demo-channel' : '#channel'), hidden: document.querySelector('#demo')?.hidden ?? null,
            bridge: text('#bridge'), marker: text('h1'), visibility: document.visibilityState, focused: document.hasFocus(),
            qualityType: typeof window.createQualityController, chatType: typeof window.mpdInstallChat,
            nativeInvokeType: typeof window.__TAURI__?.core?.invoke, audioSetterType: typeof window.mpdSetAudio,
            assets: [...document.querySelectorAll('script[src],link[rel=stylesheet][href]')].flatMap(element => {
              let asset; try { asset = new URL(element.getAttribute(element.tagName === 'SCRIPT' ? 'src' : 'href'), location.href); } catch { return []; }
              const name = asset.pathname.split('/').pop();
              if (!['chat.js', 'quality.js', 'player.js', 'style.css', 'chat.css'].includes(name)) return [];
              const session = asset.searchParams.get('e2e_session');
              return [{ name, session: /^(0|[1-9][0-9]{0,19})$/.test(session ?? '') ? session : null }];
            }).slice(0, 5),
            resources: performance.getEntriesByType('resource').flatMap(entry => {
              let name; try { name = new URL(entry.name).pathname.split('/').pop(); } catch { return []; }
              if (!['chat.js', 'quality.js', 'player.js', 'index.html', 'style.css', 'chat.css', 'fixture.js'].includes(name)) return [];
              return [{ name, duration: Math.round(entry.duration), transferSize: entry.transferSize, responseStatus: entry.responseStatus ?? null }];
            }).slice(-12) };
        }, demo);
        const record = raw !== null && typeof raw === 'object' && !Array.isArray(raw) ? raw : {};
        const short = value => typeof value === 'string' ? sanitize(value).slice(0, 160) : null;
        const snapshot = { resultType: raw === null ? 'null' : Array.isArray(raw) ? 'array' : typeof raw,
          readyState: short(record.readyState), title: short(record.title), channel: short(record.channel),
          hidden: typeof record.hidden === 'boolean' ? record.hidden : null, bridge: short(record.bridge), marker: short(record.marker),
          visibility: short(record.visibility), focused: typeof record.focused === 'boolean' ? record.focused : null,
          qualityType: short(record.qualityType), chatType: short(record.chatType), nativeInvokeType: short(record.nativeInvokeType), audioSetterType: short(record.audioSetterType),
          assets: Array.isArray(record.assets) ? record.assets.slice(0, 5).flatMap(entry =>
            ['chat.js', 'quality.js', 'player.js', 'style.css', 'chat.css'].includes(entry?.name) ? [{ name: entry.name, session: typeof entry.session === 'string' && /^(0|[1-9][0-9]{0,19})$/.test(entry.session) ? entry.session : null }] : []) : [],
          resources: Array.isArray(record.resources) ? record.resources.slice(-12).map(entry => ({ name: short(entry.name), duration: Number.isFinite(entry.duration) ? entry.duration : null,
            transferSize: Number.isFinite(entry.transferSize) ? entry.transferSize : null, responseStatus: Number.isFinite(entry.responseStatus) ? entry.responseStatus : null })) : [] };
        const ready = record.readyState === 'complete' && channels.includes(record.channel) &&
          (demo ? record.hidden === false : record.marker === 'MPD E2E local viewer');
        return { ready, channel: record.channel, snapshot };
    };
    try {
      return await until(async () => {
        const result = await sample(); observation.last = result.snapshot; observation.passed = result.ready;
        return result.ready && result.channel;
      }, 'Expected initialized native viewer document');
    } catch (error) {
      // Failure-only diagnostic: never turn this original assertion into a pass.
      if (demo) {
        observation.afterFocus = { attempted: false, recovered: false, last: null };
        try {
          const channel = new URLSearchParams(new URL(await this.browser.getUrl()).hash.slice(1)).get('channel');
          const allowed = ['bravo_demo', 'charlie_demo', 'alpha_fixture', 'bravo_fixture'];
          if (!allowed.includes(channel) || !channels.includes(channel)) throw new Error('Focus diagnostic refused an unrecognized fixture channel');
          const failedHandle = observation.label;
          const handles = await this.browser.getWindowHandles();
          if (!handles.includes(failedHandle) || !handles.includes(this.manager)) throw new Error('Focus diagnostic window no longer exists');
          await this.browser.switchToWindow(this.manager);
          const focus = await this.browser.$(`[aria-label="Focus ${channel}"]`);
          if (!await focus.isExisting()) throw new Error('Focus diagnostic manager control is missing');
          observation.afterFocus.attempted = true;
          observation.afterFocus.channel = channel;
          await focus.click(); // Existing manager UI, through the production controller.
          await this.browser.switchToWindow(failedHandle);
          let expired = false, timer;
          try {
            await Promise.race([
              (async () => {
                while (!expired) {
                  const result = await sample();
                  if (expired) return;
                  observation.afterFocus.last = result.snapshot;
                  if (result.ready) { observation.afterFocus.recovered = true; return; }
                  await delay(150);
                }
              })(),
              new Promise(resolve => { timer = setTimeout(() => { expired = true; resolve(); }, 5000); })
            ]);
          } finally { expired = true; clearTimeout(timer); }
        } catch (diagnosticError) {
          observation.afterFocus.error = sanitize(diagnosticError.message).slice(0, 512);
        }
      }
      throw new Error(`${error.message}; last observed readiness: ${JSON.stringify(observation.last)}; failure-only Focus diagnostic: ${JSON.stringify(observation.afterFocus ?? null)}`);
    }
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
  async screenshot(name, maxImages = 8) {
    if (!this.browser) return;
    let original;
    try { original = await this.browser.getWindowHandle(); } catch { return; }
    let handles;
    try { handles = await this.browser.getWindowHandles(); } catch { return; }
    let captured = 0;
    try {
      for (const [index, handle] of handles.slice(0, Math.min(8, maxImages)).entries()) {
        try {
          await this.browser.switchToWindow(handle);
          const base64 = await this.browser.takeScreenshot();
          const buffer = Buffer.from(base64, 'base64');
          if (buffer.length > 8 * 1024 * 1024 || buffer.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('Screenshot must be a bounded PNG');
          await writeFile(path.join(this.output, `${name}${name.endsWith('-w') ? '' : '-'}${index}.png`), buffer);
          captured++;
        } catch (error) { await appendFile(path.join(this.output, 'capture.log'), `${sanitize(error.message).slice(0, 1000)}\n`); }
      }
    } finally {
      try { await this.browser.switchToWindow(original); } catch {}
    }
    return captured;
  }
  async checkpoint(testName) {
    const evidence = this.checkpoints;
    evidence.attempted++;
    const sequence = String(evidence.attempted).padStart(3, '0');
    if (!evidence.enabled || !this.browser || this.cancelled || evidence.captured >= evidence.quota) {
      evidence.skipped++;
      if (evidence.cases.length < 128) evidence.cases.push({ sequence: evidence.attempted, name: testName, captured: 0, skipped: true });
      return;
    }
    const count = await this.screenshot(`checkpoint-${sequence}-w`, Math.min(8, evidence.quota - evidence.captured));
    evidence.captured += count || 0;
    if (!count) evidence.skipped++;
    if (evidence.cases.length < 128) evidence.cases.push({ sequence: evidence.attempted, name: testName, captured: count || 0, skipped: !count });
  }
  async policyProbe(scenario) {
    if (this.cancelled) throw new Error(this.cancelled);
    if (this.child) throw new Error('Stop the owned GUI before a driverless policy probe');
    const port = await freePort();
    if (this.cancelled) throw new Error(this.cancelled);
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
    if (this.child || this.cleanupChild) throw new Error('Cannot clean credentials while an owned process remains unresolved');
    const port = await freePort();
    const child = spawn(this.binary, ['--e2e-cleanup'], { shell: false, windowsHide: true, detached: process.platform !== 'win32', stdio: 'ignore', env: environment({ MPD_E2E_ROOT: root, MPD_E2E_RUN_ID: runId, TAURI_WEBDRIVER_PORT: String(port) }) });
    this.cleanupChild = child;
    let error; child.on('error', value => { error = value; });
    try {
      await until(() => { if (error) throw error; return exited(child); }, 'Scoped credential cleanup did not exit', 15000);
      if (child.exitCode !== 0) throw new Error(`Scoped credential cleanup exited ${child.exitCode}`);
    } finally {
      await terminateOwned(child);
      this.cleanupChild = null;
    }
  }
  async flushLogs() {
    let timeout;
    try {
      await Promise.race([
        (async () => { if (this.logClosed) await this.logClosed; await this.logWrites; })(),
        new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error('Application log drain did not complete')), 10000); })
      ]);
      if (this.logErrors.length) throw new Error(`Application log write failed: ${this.logErrors.join('; ')}`);
      this.logClosed = null;
    } finally { clearTimeout(timeout); }
  }
  async stop() {
    try { await this.browser?.deleteSession(); } catch {}
    this.browser = null;
    if (this.child) {
      await terminateOwned(this.child);
      this.child = null;
    }
    if (this.cleanupChild) {
      await terminateOwned(this.cleanupChild);
      this.cleanupChild = null;
    }
    await this.flushLogs();
  }
  async abortOwned() {
    // An interrupted WebDriver command can make deleteSession unsafe or
    // unbounded. Drop that client and terminate only processes we spawned.
    this.browser = null;
    if (this.child) {
      await terminateOwned(this.child);
      this.child = null;
    }
    if (this.cleanupChild) {
      await terminateOwned(this.cleanupChild);
      this.cleanupChild = null;
    }
    await this.flushLogs();
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
