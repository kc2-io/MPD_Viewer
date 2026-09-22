'use strict';
// Execute the actual preserved host and native adapter together with a fake SDK.
// This tests the protocol/startup order, not Twitch media or native IPC.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.resolve(__dirname, '..');
const html = fs.readFileSync(path.join(root, 'web/parent.mpdviewer.com/index.html'), 'utf8');
const host = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)]
  .find(m => !/\bsrc\s*=/i.test(m[1]) && m[2].trim())[2];
const adapter = fs.readFileSync(path.join(root, 'src-tauri/hosted-player-adapter.js'), 'utf8');

const qualityFactory = fs.readFileSync(path.join(root, 'player-wrapper/quality.js'), 'utf8');

function fixture({quality = 'auto', volume = 25, muted = false, session = 7, channel = 'alpha', hashOnly = false,
  frame = false, pathName = '/', configOrigin = 'https://parent.mpdviewer.com'} = {}) {
  const url = new URL(`https://parent.mpdviewer.com${pathName}`);
  if (hashOnly) url.hash = `channel=${channel}&session=${session}&volume=${volume}`;
  else url.search = new URLSearchParams({channel, active: '__mpd-native-pending__', volume: String(volume / 100), pauseInactive: 'false'});
  const listeners = [], pending = [], intervals = [], timeouts = [], instances = [], reports = [];
  class Element {
    constructor() { this.children = []; this.style = {}; this.dataset = {}; this.attributes = {}; }
    appendChild(child) { child.parent = this; this.children.push(child); return child; }
    setAttribute(key, value) { this.attributes[key] = value; }
    remove() { this.parent.children = this.parent.children.filter(c => c !== this); }
  }
  const grid = new Element(), body = new Element();
  class Player {
    constructor(id, options) { this.id = id; this.options = options; this.listeners = new Map();
      this.calls = []; this.volume = 1; this.muted = options.muted; this.paused = false; instances.push(this); }
    addEventListener(name, callback) { const list = this.listeners.get(name) || []; list.push(callback); this.listeners.set(name, list); }
    emit(name) { for (const callback of this.listeners.get(name) || []) callback(); }
    setMuted(value) { this.calls.push(['mute', value]); this.muted = value; }
    setVolume(value) { this.calls.push(['volume', value]); this.volume = value; }
    getQualities() { return this.qualities ?? ['auto']; }
    getQuality() { return this.quality ?? 'auto'; }
    setQuality(value) { this.calls.push(['quality',value]); this.quality = value; }
    isPaused() { return this.paused; }
    getMuted() { return this.muted; }
    getVolume() { return this.volume; }
    play() { this.calls.push(['play']); this.paused = false; }
    pause() { this.calls.push(['pause']); this.paused = true; }
  }
  for (const name of ['READY', 'PLAYING', 'PAUSE', 'ENDED', 'ONLINE', 'OFFLINE', 'PLAYBACK_BLOCKED']) Player[name] = name;
  const context = { URLSearchParams, location: url, Twitch: {Player},
    document: {body, visibilityState: 'visible', head: new Element(),
      getElementById: id => id === 'grid' ? grid : null,
      createElement: () => new Element(), addEventListener() {}},
    addEventListener: (name, callback, capture = false) => listeners.push({name, callback, capture}),
    postMessage: data => pending.push(data),
    setInterval: callback => intervals.push(callback), setTimeout: callback => timeouts.push(callback),
    __TAURI__: {core: {invoke: (command, payload) => { reports.push({command, ...payload}); return Promise.resolve(); }}}
  };
  context.window = context; context.parent = context; context.top = frame ? {} : context;
  vm.createContext(context);
  // Use the context's own window proxy for MessageEvent.source identity.
  const window = vm.runInContext('window', context);
  const config = {origin: configOrigin, path: '/', volume, muted, session, channel, quality};
  vm.runInContext(`(${adapter})(${JSON.stringify(config)}, ${qualityFactory})`, context);
  vm.runInContext(host, context);
  function message(data, origin = url.origin, source = window) {
    const event = {data, origin, source, stopped: false, stopImmediatePropagation() { this.stopped = true; }};
    for (const listener of [...listeners].sort((a,b) => Number(b.capture) - Number(a.capture))) {
      if (listener.name === 'message' && !event.stopped) listener.callback(event);
    }
    return event;
  }
  const flush = () => { while (pending.length) message(pending.shift()); };
  flush();
  return {context, instances, reports, grid, body, intervals, timeouts, message, flush,
    ready() { instances[0].emit('READY'); flush(); },
    event(name) { instances[0].emit(name); flush(); }};
}

test('regression: fragment-only host stays empty; query adapter mounts exactly assigned channel', () => {
  assert.equal(fixture({hashOnly: true}).instances.length, 0);
  const f = fixture();
  assert.equal(f.instances.length, 1);
  assert.equal(f.instances[0].options.channel, 'alpha');
  assert.equal(f.instances[0].options.parent[0], 'parent.mpdviewer.com');
  assert.equal(f.instances[0].options.autoplay, true);
  assert.equal(f.instances[0].muted, true);
  assert.equal(f.context.active, null);
});

for (const muted of [true, false]) test(`READY applies fractional volume before requested mute=${muted}`, () => {
  const f = fixture({muted}); f.ready();
  const p = f.instances[0];
  assert.deepEqual(p.calls, [['mute', true], ['volume', 0.25], ['mute', muted]]);
  assert.equal(p.volume, 0.25);
  assert.equal(p.muted, muted);
  assert.equal(f.reports.at(-1).report.session, 7);
  assert.equal(f.reports.at(-1).report.state, 'ready');
  assert.equal(f.reports.at(-1).command, 'player_report');
});

test('audio arriving before READY is queued and latest values win', () => {
  const f = fixture();
  f.context.mpdSetAudio(80, false); f.context.mpdSetAudio(10, true);
  assert.deepEqual(f.instances[0].calls, []);
  f.ready();
  assert.deepEqual(f.instances[0].calls, [['mute', true], ['volume', 0.1], ['mute', true]]);
});

test('manual pause and autoplay block survive audio changes and periodic reports', () => {
  const f = fixture(); f.ready();
  const p = f.instances[0]; p.pause(); f.event('PAUSE');
  f.context.mpdSetAudio(55, false); f.context.mpdSetAudio(0, true);
  for (const callback of f.intervals) callback();
  assert.equal(p.paused, true);
  assert.equal(f.reports.at(-1).report.state, 'paused');
  f.event('PLAYBACK_BLOCKED'); f.context.mpdSetAudio(20, false);
  assert.equal(f.reports.at(-1).report.state, 'blocked');
  assert.equal(p.calls.filter(c => c[0] === 'play').length, 0);
});

test('zero volume and mute are independent; invalid audio input is rejected', () => {
  const f = fixture(); f.ready();
  f.context.mpdSetAudio(0, false);
  assert.equal(f.instances[0].muted, false);
  const before = f.instances[0].calls.length;
  for (const [v,m] of [[NaN,true],[Infinity,true],[-1,true],[101,true],[20,'false']]) f.context.mpdSetAudio(v,m);
  assert.equal(f.instances[0].calls.length, before);
});

test('ONLINE does not report playing; only PLAYING changes readiness to playback', () => {
  const f = fixture(); f.ready(); f.event('ONLINE');
  assert.equal(f.reports.at(-1).report.state, 'ready');
  f.event('PLAYING'); assert.equal(f.reports.at(-1).report.state, 'playing');
});

test('foreign origins, iframe sources and unrelated channels cannot supply native reports', () => {
  const f = fixture(); f.ready(); const before = f.reports.length;
  const payload = {type: 'event', event: 'PLAYING', channel: 'alpha', session: 999};
  f.message(payload, 'https://evil.example');
  f.message(payload, 'https://parent.mpdviewer.com', {});
  f.message({...payload, channel: 'beta'});
  f.message({...payload, event: 'toString'});
  assert.equal(f.reports.length, before);
  f.message(payload);
  assert.equal(f.reports.at(-1).report.session, 7);
});

test('legacy commands are blocked before the hosted listener; SDK traffic passes', () => {
  const f = fixture(); f.ready();
  for (const command of [{type: 'add', channel: 'beta'}, {type: 'remove', channel: 'alpha'},
    {type: 'setChannels', channels: ['beta']}, {type: 'setActive', channel: 'alpha'},
    {type: 'setVolume', volume: 1}, {type: 'pauseInactive', value: true}, {type: 'ping'}]) {
    for (const message of [command, JSON.stringify(command)]) assert.equal(f.message(message).stopped, true);
  }
  assert.equal(f.instances.length, 1);
  assert.equal(f.grid.children.length, 1);
  assert.equal(f.context.active, null);
  assert.equal(f.context.pauseInactive, false);
  assert.equal(f.message(JSON.stringify({type: 'add', channel: 'beta', padding: 'x'.repeat(5000)})).stopped, true);
  assert.equal(f.instances.length, 1);
  assert.equal(f.message({namespace: 'twitch-player', eventName: 'playing'}, 'https://player.twitch.tv', {}).stopped, false);
  assert.equal(f.message(JSON.stringify({namespace: 'twitch-player', padding: 'x'.repeat(5000)}), 'https://player.twitch.tv', {}).stopped, false);
});

test('unavailable native telemetry does not prevent audio initialization', () => {
  const f = fixture();
  f.context.__TAURI__.core.invoke = () => { throw new Error('unavailable'); };
  assert.doesNotThrow(() => f.ready());
  assert.equal(f.instances[0].volume, 0.25);
});

test('adapter is restricted to the top-level configured origin and path', () => {
  for (const options of [{frame: true}, {pathName: '/other'}, {configOrigin: 'https://wrong.example'}]) {
    const f = fixture(options);
    assert.equal(typeof f.context.mpdSetAudio, 'undefined');
    assert.equal(f.intervals.length, 0);
  }
});

test('initialization timeout is visible, reports error, and never starts or reloads playback', () => {
  const f = fixture(); f.timeouts.forEach(callback => callback());
  assert.equal(f.body.children[0].attributes.role, 'alert');
  assert.match(f.body.children[0].textContent, /Twitch did not initialize/);
  assert.equal(f.reports.at(-1).report.state, 'error');
  assert.equal(f.instances[0].calls.length, 0);
  f.ready(); assert.equal(f.body.children.length, 0);
});

test('initialization notice is app-themed across both palettes without touching the host', () => {
  const f = fixture(); f.timeouts.forEach(callback => callback());
  const style = f.context.document.head.children[0];
  assert.ok(style && style.textContent.includes('@media (prefers-color-scheme:light)'));
  assert.ok(style.textContent.includes('#mpd-adapter-notice{'));
  const notice = f.body.children[0];
  assert.equal(notice.attributes.id, 'mpd-adapter-notice');
  assert.equal(notice.attributes.role, 'alert');
  assert.equal(f.context.document.head.children.length, 1);
  assert.equal(f.grid.children.length, 1);
});


test('hosted quality uses nearest advertised variant without recreating or playing', () => {
  const f=fixture({quality:'180p'}), p=f.instances[0];
  p.qualities=['auto','360p30','720p60','chunked']; f.ready();
  assert.equal(p.quality,'360p30');
  f.context.mpdSetQuality('720p');
  assert.equal(p.quality,'720p60');
  assert.equal(f.instances.length,1);
  assert.equal(p.calls.filter(c=>c[0]==='play').length,0);
});
test('latest native preference wins before READY and after document recreation', () => {
  for(const openingPreference of ['auto','180p']) {
    const f=fixture({quality:openingPreference}), p=f.instances[0];
    p.qualities=['auto','360p30','720p60'];
    f.context.mpdSetQuality('720p'); f.ready();
    assert.equal(p.quality,'720p60');
  }
  // A reloaded document receives the latest Rust preference on its READY report.
  const reloaded=fixture({quality:'180p'}), p=reloaded.instances[0];
  p.qualities=['auto','360p30','720p60']; reloaded.ready();
  assert.equal(p.quality,'360p30');
  reloaded.context.mpdSetQuality('720p');
  assert.equal(p.quality,'720p60');
});
test('hosted pause/block retain pending quality until normal playback resumes', () => {
  const f=fixture({quality:'180p'}), p=f.instances[0];
  p.qualities=['auto','360p30','720p60']; f.ready();
  p.pause(); f.event('PAUSE'); f.context.mpdSetQuality('720p');
  for(const tick of f.intervals)tick();
  assert.equal(p.quality,'360p30'); assert.equal(p.paused,true);
  f.event('PLAYBACK_BLOCKED'); p.paused=false;
  for(const tick of f.intervals)tick();
  assert.equal(p.quality,'360p30');
  f.event('PLAYING'); assert.equal(p.quality,'720p60');
  assert.equal(p.calls.filter(c=>c[0]==='play').length,0);
});
test('late hosted variants reselect once and audio does not undo preference', () => {
  const f=fixture({quality:'180p'}), p=f.instances[0];
  p.qualities=[]; f.ready();
  p.qualities=['auto','360p30','chunked'];
  for(const tick of f.intervals)tick();
  assert.equal(p.quality,'360p30');
  f.context.mpdSetAudio(50,true);
  for(const tick of f.intervals)tick();
  assert.equal(p.calls.filter(c=>c[0]==='quality').length,1);
});
