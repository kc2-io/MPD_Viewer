const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../player-wrapper/chat.js'), 'utf8');
function fixture(overrides = {}, url = 'https://parent.mpdviewer.com/?channel=alpha', loading = false) {
  const nodes = [], timeouts = new Map(), events = {};
  class Element {
    constructor(tag) { this.tag = tag; this.children = []; this.hidden = false; this.dataset = {}; this.attrs = {}; this.events = {}; this.classList = {add(){}}; nodes.push(this); }
    append(...items) { for (const item of items) { item.parentElement = this; this.children.push(item); } }
    setAttribute(k,v) { this.attrs[k] = v; }
    addEventListener(k,v) { this.events[k] = v; }
  }
  const body = new Element('body'), grid = new Element('div'); grid.id = 'grid'; body.append(grid);
  const video = new Element('iframe'); grid.append(video);
  const player = {paused:true, muted:true, volume:0.25, plays:0};
  const context = { URL, URLSearchParams, location:new URL(url),
    document: {body, head:new Element('head'), documentElement:new Element('html'), readyState:loading?'loading':'complete',
      getElementById:id=>nodes.find(n=>n.id===id), createElement:tag=>new Element(tag), addEventListener:(k,v)=>events[k]=v},
    setTimeout:fn=>{const id=timeouts.size+1;timeouts.set(id,fn);return id;}, clearTimeout:id=>timeouts.delete(id) };
  context.window=context; context.top=context;
  vm.createContext(context);
  const config = {channel:'alpha', mode:'hosted', origin:'https://parent.mpdviewer.com', path:'/', css:'', demo:false, ...overrides};
  const run=()=>vm.runInContext(`(${source})(${JSON.stringify(config)})`,context);
  run();
  return {nodes, grid, video, player, body, context, events, timeouts, run, get:id=>nodes.find(n=>n.id===id)};
}
test('official chat uses assigned login and real parent hostname',()=>{
  const f=fixture(); const frame=f.get('mpd-chat-panel').children.find(n=>n.tag==='iframe');
  assert.equal(frame.src,'https://www.twitch.tv/embed/alpha/chat?parent=parent.mpdviewer.com');
  assert.equal(f.get('mpd-chat-panel').hidden,false);
});
test('rejects injected channel, foreign assignment, duplicate channel, origin or path',()=>{
  for(const [config,url] of [ [{channel:'a/../../evil'}], [{channel:'beta'}], [{},'https://parent.mpdviewer.com/?channel=alpha&channel=beta'],
    [{origin:'https://evil.example'}], [{path:'/other'}]]) {
    assert.equal(fixture(config,url).get('mpd-chat-panel'),undefined);
  }
});
test('collapse/restore retains video ancestry, chat frame, and playback state',()=>{
  const f=fixture(); const panel=f.get('mpd-chat-panel'), frame=panel.children.at(-1), toggle=f.get('mpd-chat-toggle');
  for(let i=0;i<6;i++) toggle.events.click();
  assert.equal(panel.hidden,false); assert.equal(toggle.attrs['aria-expanded'],'true');
  assert.equal(frame,panel.children.at(-1)); assert.equal(f.video.parentElement,f.grid); assert.equal(f.grid.parentElement,f.body);
  assert.deepEqual(f.player,{paused:true,muted:true,volume:0.25,plays:0});
  f.run(); assert.equal(f.nodes.filter(n=>n.id==='mpd-chat-panel').length,1);
});
test('demo builds simulated transcript without chat iframe or timer',()=>{
  const f=fixture({demo:true});
  assert.equal(f.get('mpd-chat-panel').children.some(n=>n.tag==='iframe'),false);
  assert.equal(f.timeouts.size,0);
});
test('load, timeout and failure update only local chat feedback',()=>{
  const f=fixture(), status=f.get('mpd-chat-status'), frame=f.get('mpd-chat-panel').children.at(-1);
  [...f.timeouts.values()][0](); assert.match(status.textContent,/longer/);
  frame.events.load(); assert.equal(status.hidden,true); assert.equal(f.timeouts.size,0);
  frame.events.error(); assert.equal(status.hidden,false); assert.match(status.textContent,/could not load/);
  assert.equal(f.video.parentElement,f.grid);
});
test('document-start injection waits for host DOM and honors top-frame boundary',()=>{
  const f=fixture({},undefined,true); assert.equal(f.get('mpd-chat-panel'),undefined);
  f.events.DOMContentLoaded(); assert.ok(f.get('mpd-chat-panel'));
  const child=fixture({},undefined,true); child.context.top={}; child.events.DOMContentLoaded=undefined; child.run();
  assert.equal(child.get('mpd-chat-panel'),undefined); assert.equal(child.events.DOMContentLoaded,undefined);
});
test('replacement document derives chat from its new assignment',()=>{
  const f=fixture({channel:'beta'},'https://parent.mpdviewer.com/?channel=beta');
  assert.equal(f.get('mpd-chat-panel').children.at(-1).src,'https://www.twitch.tv/embed/beta/chat?parent=parent.mpdviewer.com');
});
