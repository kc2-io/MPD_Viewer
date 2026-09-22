// Focused DM-010 coverage: computed app-owned theme colors under OS light/dark.
// Everything external (Twitch embed, beacon, fonts) is mocked by per-context
// routes so this suite is deterministic and offline; the external-request log
// is asserted against an explicit allowlist per target. Live media switches
// assert live theming without reload. This is browser evidence only, not native
// OS-theme evidence; live Twitch behavior stays in the separately labeled
// feasibility/native acceptance records.
// Requires Playwright resolvable via NODE_PATH (see tests/chat.browser.cjs).
const {chromium} = require('playwright');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const chatCss = fs.readFileSync(path.join(root, 'player-wrapper', 'chat.css'), 'utf8');
const chatJs = fs.readFileSync(path.join(root, 'player-wrapper', 'chat.js'), 'utf8');
// Explicit allowlist of third-party hosts the two simulators may reference.
// Everything is still mocked locally; this only proves no unplanned host is
// contacted. Local server bases are always allowed.
const EXTERNAL_ALLOW = ['.twitch.tv', '.twitchcdn.net', '.ttvnw.net', '.cloudflareinsights.com'];
function parse(color){
  if (!color) throw new Error(`missing color: ${color}`);
  let m = /^#([0-9a-f]{6})$/i.exec(color.trim());
  if (m) return [0, 1, 2].map(i => parseInt(m[1].slice(i * 2, i * 2 + 2), 16));
  m = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/.exec(color);
  assert.ok(m, `unexpected color: ${color}`);
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}
function luminance(rgb){
  const [r,g,b] = rgb.map(c => { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function ratio(aChar,bChar){
  const [x,y] = [luminance(parse(aChar)), luminance(parse(bChar))].sort((p,q)=>q-p);
  return (x + 0.05) / (y + 0.05);
}
function assertAA(fg,bg,label,text=true,large=false){
  let r;
  try { r = ratio(fg,bg); }
  catch (e) { throw new Error(`${label}: fg=${fg} bg=${bg} :: ${e.message}`); }
  assert.ok(r >= (text ? (large ? 3 : 4.5) : 3), `${label} contrast ${r.toFixed(2)} (need ${text ? (large?3:4.5) : 3})`);
}
function serve(dirRel){
  const dir = path.join(root, dirRel);
  return new Promise(resolve => {
    const s = http.createServer((req,res) => {
      const name = new URL(req.url, 'http://localhost').pathname.slice(1) || 'index.html';
      if (name.includes('/') || name === 'favicon.ico') { res.writeHead(404).end(); return; }
      const file = path.join(dir, name);
      if (!file.startsWith(dir + path.sep) && file !== dir) { res.writeHead(403).end(); return; }
      if (!fs.existsSync(file)) { res.writeHead(404).end(); return; }
      const ext = name.split('.').pop();
      res.setHeader('Content-Type', ext === 'js' ? 'text/javascript' : ext === 'css' ? 'text/css' : 'text/html');
      res.end(fs.readFileSync(file));
    });
    s.listen(0, '127.0.0.1', () => resolve(s));
  });
}
const readFile = {manager: 'ui', wrapper: 'player-wrapper', hosted: 'web/parent.mpdviewer.com'};
(async()=>{
  const manager = await serve(readFile.manager);
  const wrapper = await serve(readFile.wrapper);
  const hosted = await serve(readFile.hosted);
  const base = {
    manager: `http://localhost:${manager.address().port}/index.html`,
    wrapper: `http://localhost:${wrapper.address().port}/index.html#channel=alpha&session=7&demo=false&volume=25&muted=true`,
    hosted: `http://localhost:${hosted.address().port}/index.html?channel=alpha`,
  };
  const localOrigins = [0, 1, 2].map(i => new URL(Object.values(base)[i]).origin).filter((v, i, a) => a.indexOf(v) === i);
  const browser = await (async()=>{ try { return await chromium.launch({channel:'msedge',headless:true}); } catch { return chromium.launch({headless:true}); } })();
  try {
    for (const schema of ['manager', 'wrapper', 'hosted']) {
      const context = await browser.newContext();
      const external = [];
      await context.route('**/*', route => {
        const req = route.request().url();
        if (!/^https?:$/.test(new URL(req).protocol)) return route.abort();
        const origin = new URL(req).origin;
        if (localOrigins.includes(origin)) return route.continue();
        external.push(req);
        return route.fulfill({status: 200, contentType: 'text/html', body: '<!doctype html><html><body></body></html>'});
      });
      if (schema === 'hosted') {
        // Mirror the native initialization_script: chat.js + config on the
        // preserved hosted document. The preserved file itself is not edited.
        const hostedUrl = new URL(base.hosted);
        const config = {mode: 'hosted', origin: hostedUrl.origin, path: hostedUrl.pathname, channel: 'alpha', demo: false, css: chatCss};
        await context.addInitScript({content: `${chatJs}\n;mpdInstallChat(${JSON.stringify(config)});`});
      }
      const page = await context.newPage();
      const waitPanel = schema !== 'manager';
      const collect = async (pc, scheme) => {
        await pc.emulateMedia({colorScheme: scheme});
        await pc.goto(base[schema], {waitUntil: 'domcontentloaded'});
        if (waitPanel) await pc.locator('#mpd-chat-panel').waitFor();
        await pc.waitForTimeout(150);
        return pc.evaluate(() => {
          const q = s => document.querySelector(s);
          const cs = el => el && getComputedStyle(el);
          const color = (sel, prop) => { const el = q(sel); return el ? cs(el)[prop] : null; };
          const tokens = Object.fromEntries(['--bg','--panel','--panel-2','--topbar','--line','--text','--muted','--green','--pill','--pill-text','--pill-on','--ctrl-line','--panel-line','--field-bg','--tag-line','--tag-text','--error-bg','--error-line','--error-text','--auth-bg','--auth-line','--footer','--demo-high','--demo-low']
            .map(t => [t, getComputedStyle(document.documentElement).getPropertyValue(t).trim()]));
          const focusProbe = (sel) => {
            const el = q(sel); if (!el) return null;
            el.focus();
            const c = cs(el);
            return {bg: c.backgroundColor, border: c.borderTopColor, outline: c.outlineColor, color: c.color};
          };
          return {
            schemeDeclared: getComputedStyle(document.documentElement).colorScheme,
            tokens,
            bodyBg: color('body', 'backgroundColor'), htmlBg: color('html', 'backgroundColor'),
            bodyColor: color('body', 'color'),
            primaryBg: color('.primary', 'backgroundColor'), primaryColor: color('.primary', 'color'),
            runningPillBg: color('.pill.running', 'backgroundColor'), runningPillColor: color('.pill.running', 'color'),
            pillBg: color('.pill', 'backgroundColor'), pillColor: color('.pill', 'color'),
            errorBox: color('.error', 'backgroundColor'), errorText: color('.error', 'color'), errorBorder: color('.error', 'borderTopColor'),
            authBox: color('.device-auth', 'backgroundColor'), authColor: color('.device-auth', 'color'), authBorder: color('.device-auth', 'borderTopColor'),
            tagLine: color('.tag', 'borderTopColor'), tagText: color('.tag', 'color'),
            button: focusProbe('#refresh'), input: focusProbe('#channel'), select: focusProbe('#quality'),
            footer: q('footer') ? cs(q('footer')).color : null,
            demoLabel: color('#demo .eyebrow', 'color'),
            // wrapper / hosted chrome
            videoBg: color('#video', 'backgroundColor'), panelBg: color('#mpd-chat-panel', 'backgroundColor'),
            noteColor: color('#mpd-chat-status, .mpd-chat-note', 'color'),
            toolbarButton: focusProbe('.mpd-chat-toolbar button'),
            chatLayoutBg: color('.mpd-chat-layout', 'backgroundColor'),
            host: location.hostname, pathname: location.pathname,
            gridParentId: (() => { const g = q('#grid'); return g ? (g.parentElement ? g.parentElement.id || g.parentElement.tagName : null) : null; })(),
            chatFrames: document.querySelectorAll('#mpd-chat-panel iframe').length,
            collapsed: q('#mpd-chat-panel') ? q('#mpd-chat-panel').hidden : null,
          };
        });
      };
      const d = await collect(page, 'dark');
      const l = await collect(page, 'light');
      assert.ok(d.schemeDeclared.includes('light') && d.schemeDeclared.includes('dark'));
      if (schema !== 'hosted') assert.ok(d.tokens['--bg'] !== l.tokens['--bg']);
      const assertManagers = (which) => {
        const t = which.tokens;
        assertAA(which.bodyColor, t['--field-bg'], 'manager input text');
        assertAA(t['--ctrl-line'], t['--field-bg'], 'manager input/select boundary', false);
        assertAA(t['--ctrl-line'], '#ffffff', 'manager control boundary on white', false);
        assertAA(which.button.border, which.button.bg, 'manager button boundary', false);
        assertAA(which.button.outline, which.button.bg, 'manager button focus indicator', false);
        assertAA(which.input.border, which.input.bg, 'manager input boundary', false);
        assertAA(which.input.outline, which.input.bg, 'manager input focus indicator', false);
        assertAA(which.select.outline, which.select.bg, 'manager select focus indicator', false);
        assertAA(which.errorText, which.errorBox, 'manager error text');
        assertAA(which.errorBorder, which.errorBox, 'manager error boundary', false);
        assertAA(which.authColor, which.authBox, 'manager auth text');
        assertAA(which.authBorder, which.authBox, 'manager auth boundary', false);
        assertAA(t['--panel-line'], '#ffffff', 'manager panel-line on white', false);
        assertAA(t['--panel-line'], t['--panel-2'], 'manager panel-line on panel-2', false);
        assertAA(t['--pill-text'], t['--pill'], 'manager pill text');
        assertAA(t['--green'], t['--pill-on'], 'manager running status');
        assertAA(t['--tag-text'], t['--topbar'], 'manager tag text on topbar');
        assertAA(t['--tag-line'], t['--topbar'], 'manager tag boundary on topbar', false);
      };
      if (schema === 'manager') {
        for (const [phase, w] of [['dark', d], ['light', l]]) {
          assertAA(w.bodyColor, w.bodyBg, `manager body ${phase}`);
          assertAA(w.primaryColor, w.primaryBg, `manager primary ${phase}`);
          assertAA(w.footer, w.bodyBg, `manager footer ${phase}`);
          assertManagers(w);
        }
        if (external.length) console.error('manager external:', external);
        assert.equal(external.length, 0, 'manager palette test must not request external hosts');
      } else if (schema === 'wrapper') {
        for (const [phase, w] of [['dark', d], ['light', l]]) {
          assertAA(w.bodyColor, w.htmlBg, `wrapper body ${phase}`);
          assertAA(w.demoLabel, w.tokens['--demo-high'], `demo label gradient high ${phase}`);
          assertAA(w.demoLabel, w.tokens['--demo-low'], `demo label gradient low ${phase}`);
          assert.equal(w.videoBg, 'rgb(5, 10, 13)', 'wrapper video letterbox stays dark');
          assertAA(w.toolbarButton.border, w.toolbarButton.bg, `wrapper toolbar boundary ${phase}`, false);
          assertAA(w.toolbarButton.outline, w.toolbarButton.bg, `wrapper toolbar focus ${phase}`, false);
          assertAA(w.noteColor, w.chatLayoutBg, `wrapper chat note ${phase}`);
        }
        for (const url of external) {
          const host = new URL(url).hostname;
          assert.ok(EXTERNAL_ALLOW.some(s => host === s.slice(1) || host.endsWith(s)), `unexpected external host in wrapper: ${url}`);
        }
      } else {
        for (const [phase, w] of [['dark', d], ['light', l]]) {
          assert.equal(w.chatLayoutBg, phase === 'dark' ? 'rgb(10, 17, 22)' : 'rgb(242, 245, 247)', `hosted layout ${phase}`);
          assertAA(w.toolbarButton.border, w.toolbarButton.bg, `hosted toolbar boundary ${phase}`, false);
          assertAA(w.toolbarButton.outline, w.toolbarButton.bg, `hosted toolbar focus ${phase}`, false);
          assertAA(w.noteColor, w.chatLayoutBg, `hosted chat note ${phase}`);
          assert.equal(w.gridParentId, 'BODY', 'hosted video identity retained');
          assert.equal(w.chatFrames, 1, 'hosted single chat frame per theme');
          assert.equal(w.collapsed, false, 'hosted panel visibility retained');
        }
        for (const url of external) {
          const host = new URL(url).hostname;
          assert.ok(EXTERNAL_ALLOW.some(s => host === s.slice(1) || host.endsWith(s)), `unexpected external host in hosted: ${url}`);
        }
      }
      // Live OS theme switch without reload: chrome recolors and video identity persists.
      const live = await context.newPage();
      await live.emulateMedia({colorScheme: 'light'});
      await live.goto(base[schema], {waitUntil: 'domcontentloaded'});
      if (waitPanel) await live.locator('#mpd-chat-panel').waitFor();
      const liveBefore = schema === 'hosted'
        ? await live.evaluate(() => getComputedStyle(document.querySelector('.mpd-chat-layout')).backgroundColor)
        : await live.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--bg').trim());
      await live.evaluate(() => {
        window.themeIdentity = {
          document,
          video: document.querySelector('#grid, #video'),
          chat: document.querySelector('#mpd-chat-panel iframe'),
        };
      });
      for (const phase of ['dark', 'light']) {
        await live.emulateMedia({colorScheme: phase});
        const expected = schema === 'hosted'
          ? (phase === 'dark' ? 'rgb(10, 17, 22)' : 'rgb(242, 245, 247)')
          : (phase === 'dark' ? d.tokens['--bg'] : l.tokens['--bg']);
        await live.waitForFunction(({expected, schema}) => {
          const actual = schema === 'hosted'
            ? getComputedStyle(document.querySelector('.mpd-chat-layout')).backgroundColor
            : getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
          return actual === expected;
        }, {expected, schema});
        assert.equal(await live.evaluate(() => {
          const before = window.themeIdentity;
          return before.document === document
            && before.video === document.querySelector('#grid, #video')
            && before.chat === document.querySelector('#mpd-chat-panel iframe');
        }), true, `${schema}: document/video/chat nodes survive ${phase}`);
      }
      assert.ok(liveBefore, `${schema}: initial light color exists`);
      console.log(`PASS ${schema}: both palettes, AA bounds + focus indicators, live switch without reload, offline routes`);
    }
  } finally {
    await browser.close(); manager.close(); wrapper.close(); hosted.close();
  }
})().catch(e => { console.error('FAILED:', e.message || e); process.exitCode = 1; });
