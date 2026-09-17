'use strict';
// Characterization of the supplied source, including known defects. These tests
// do not certify correctness or execute Twitch, Cloudflare, Tauri, or a browser.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '..');
const snapshot = path.join(root, 'web/parent.mpdviewer.com');
const html = fs.readFileSync(path.join(snapshot, 'index.html'), 'utf8');
const scriptTags = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)];
const inline = scriptTags.filter(m => !/\bsrc\s*=/i.test(m[1]) && m[2].trim());
assert.equal(inline.length, 1, 'Expected exactly one inline application script');

function fixture(url = 'https://parent.mpdviewer.com/') {
  const parsed = new URL(url);
  const events = new Map();
  const outbound = [];
  const invokes = [];
  const instances = [];
  class Element {
    constructor(tag) { this.tag = tag; this.children = []; this.style = {}; this.dataset = {}; }
    appendChild(child) {
      if (child.parent) child.remove();
      child.parent = this;
      this.children.push(child);
      return child;
    }
    remove() {
      if (this.parent) this.parent.children = this.parent.children.filter(c => c !== this);
      this.parent = null;
    }
  }
  const grid = new Element('div');
  class Player {
    constructor(id, options) {
      this.id = id;
      this.options = options;
      this.listeners = new Map();
      this.calls = [];
      this.ready = false;
      this.paused = true;
      this.muted = options.muted;
      instances.push(this);
    }
    addEventListener(name, callback) { this.listeners.set(name, callback); }
    emit(name) {
      if (name === 'READY') this.ready = true;
      this.listeners.get(name)?.();
    }
    record(name, value) {
      if (!this.ready) throw new Error('Simulated SDK not ready');
      this.calls.push([name, value]);
    }
    setMuted(value) { this.record('setMuted', value); this.muted = value; }
    setVolume(value) { this.record('setVolume', value); this.volume = value; }
    play() { this.record('play'); this.paused = false; }
    pause() { this.record('pause'); this.paused = true; }
  }
  for (const name of ['READY', 'PLAYING', 'PAUSE', 'ENDED', 'ONLINE', 'OFFLINE', 'PLAYBACK_BLOCKED']) {
    Player[name] = name;
  }
  const context = {
    URLSearchParams,
    location: { hostname: parsed.hostname, origin: parsed.origin, search: parsed.search, hash: parsed.hash },
    document: { getElementById: id => id === 'grid' ? grid : null, createElement: tag => new Element(tag) },
    Twitch: { Player },
    addEventListener: (name, handler) => events.set(name, handler),
    // Capture outbound traffic only. No native IPC is actually invoked.
    postMessage: (data, target) => outbound.push({ transport: 'window', data, target }),
    chrome: { webview: { postMessage: data => outbound.push({ transport: 'webview2', data }) } },
    ipc: { postMessage: data => outbound.push({ transport: 'raw-ipc', data }) },
    __TAURI__: { core: { invoke: (...args) => { invokes.push(args); return Promise.resolve(); } } },
  };
  context.window = context;
  context.parent = context;
  vm.createContext(context);
  new vm.Script(inline[0][2], { filename: 'supplied-hosted-player.inline.js' }).runInContext(context, { timeout: 1000 });
  return {
    context, grid, instances, outbound, invokes,
    readyAll: () => instances.forEach(p => p.emit('READY')),
    message: (data, origin = parsed.origin, source = context) => events.get('message')?.({ data, origin, source }),
  };
}

const keyList = object => Object.keys(object);
const cellOrder = f => f.grid.children.map(c => c.dataset.channel);

test('snapshot: contains one inline application plus Twitch and Cloudflare scripts', () => {
  assert.equal(scriptTags.length, 3);
  assert.ok(scriptTags.some(m => m[1].includes('https://player.twitch.tv/js/embed/v1.js')));
  assert.ok(scriptTags.some(m => m[1].includes('https://static.cloudflareinsights.com/')));
  assert.ok(!/\\[._|<>]/.test(inline[0][2]), 'Markdown escapes must not remain');
});

test('snapshot integrity: manifest hash agrees with reconstructed HTML', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(snapshot, 'SOURCE.json'), 'utf8'));
  assert.equal(manifest.files[0].sha256, crypto.createHash('sha256').update(html).digest('hex'));
  assert.equal(manifest.live_fetch_verified, false);
});

test('defaults: empty root boot, 50 percent volume and pauseInactive enabled', () => {
  const f = fixture();
  assert.equal(f.instances.length, 0);
  assert.equal(f.context.volume, 0.5);
  assert.equal(f.context.pauseInactive, true);
  assert.ok(f.outbound.some(x => x.data?.type === 'hostReady'));
});

test('query bootstrap creates channels and derives parent from actual hostname', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta&active=beta&volume=0.25');
  f.readyAll();
  assert.deepEqual(keyList(f.context.players), ['alpha', 'beta']);
  assert.equal(f.context.active, 'beta');
  assert.equal(f.context.players.beta.player.volume, 0.25);
  assert.equal(f.instances[0].options.parent[0], 'parent.mpdviewer.com');
});

test('known incompatibility: uploaded POC fragment does not bootstrap any players', () => {
  const f = fixture('https://parent.mpdviewer.com/#channel=alpha&session=1&volume=25&muted=false&demo=false');
  assert.equal(f.instances.length, 0);
});

test('known incompatibility: query volume=25 clamps to 1 rather than 25 percent', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&volume=25');
  f.readyAll();
  assert.equal(f.context.volume, 1);
  assert.equal(f.context.players.alpha.player.volume, 1);
});

test('default behavior: only active channel is playing and unmuted after READY', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta&channel=gamma');
  f.readyAll();
  assert.equal(f.context.players.alpha.player.paused, false);
  assert.equal(f.context.players.alpha.player.muted, false);
  for (const name of ['beta', 'gamma']) {
    assert.equal(f.context.players[name].player.paused, true);
    assert.equal(f.context.players[name].player.muted, true);
  }
});

test('pauseInactive=false at bootstrap requests SDK autoplay for each constructed player', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta&pauseInactive=false');
  assert.ok(f.instances.every(p => p.options.autoplay === true));
  // The mock does not simulate browser autoplay, visibility, or actual playback.
});

test('known behavior: disabling pauseInactive later does not resume an already paused channel', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta');
  f.readyAll();
  const beta = f.context.players.beta.player;
  const playsBefore = beta.calls.filter(c => c[0] === 'play').length;
  f.message({ type: 'pauseInactive', value: false });
  assert.equal(beta.paused, true);
  assert.equal(beta.calls.filter(c => c[0] === 'play').length, playsBefore);
});

test('known behavior: volume change resumes a manually paused active player', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha');
  f.readyAll();
  const alpha = f.context.players.alpha.player;
  alpha.pause();
  f.message({ type: 'setVolume', volume: 0.2 });
  assert.equal(alpha.paused, false);
});

test('known defect: second add lays out using one registered player', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta');
  assert.equal(f.grid.children.length, 2);
  assert.equal(f.grid.style.gridTemplateColumns, 'repeat(1, 1fr)');
});

test('retained setChannels entries keep their player instance but are not reordered', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta&channel=gamma');
  const alpha = f.context.players.alpha.player;
  f.message({ type: 'setChannels', channels: ['gamma', 'beta', 'alpha'] });
  assert.equal(f.context.players.alpha.player, alpha);
  assert.deepEqual(cellOrder(f), ['alpha', 'beta', 'gamma']);
});

test('known gap: removing active channel alone leaves no active replacement', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha&channel=beta');
  f.readyAll();
  f.message({ type: 'remove', channel: 'alpha' });
  assert.equal(f.context.active, null);
  assert.equal(f.context.players.beta.player.paused, true);
});

test('known security gap: inbound command accepts an unrelated origin and source', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha');
  f.message({ type: 'remove', channel: 'alpha' }, 'https://untrusted.example', {});
  assert.equal(keyList(f.context.players).length, 0);
});

test('known validation gap: nonnumeric volume produces NaN', () => {
  const f = fixture();
  f.message({ type: 'setVolume', volume: 'not-a-number' });
  assert.ok(Number.isNaN(f.context.volume));
});

test('known validation gap: nonarray setChannels throws', () => {
  const f = fixture();
  assert.throws(() => f.message({ type: 'setChannels', channels: 'alpha' }), /map is not a function/);
});

test('known protocol mismatch: no POC audio function or Tauri command invocation', () => {
  const f = fixture('https://parent.mpdviewer.com/?channel=alpha');
  f.readyAll();
  assert.equal(typeof f.context.mpdSetAudio, 'undefined');
  assert.equal(f.invokes.length, 0);
  assert.ok(f.outbound.some(x => x.transport === 'raw-ipc'));
  const event = f.outbound.find(x => x.data?.type === 'event').data;
  assert.equal(event.session, undefined);
});

test('snapshot has no chat iframe or local dependency files', () => {
  assert.ok(!/<iframe\b/i.test(html));
  assert.ok(!/embed\/.*\/chat|Twitch\.Embed/.test(html));
  assert.ok(!/<(?:script|link)\b[^>]*(?:src|href)=["'](?:player\.js|style\.css)/i.test(html));
});
