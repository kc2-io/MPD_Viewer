// Pure quality selection tests with a fake official-player SDK surface.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname,'../player-wrapper/quality.js'),'utf8');
const create = vm.runInNewContext(`(${source})`);
function fixture(preference='auto',qualities=['auto','160p30','360p30','720p30','chunked']) {
  const calls=[];
  const sdk={qualities,paused:false,fail:false,
    getQualities(){return this.qualities;},
    getQuality(){return this.currentQuality;},
    isPaused(){return this.paused;},
    setQuality(value){calls.push(value);if(this.fail)throw new Error('SDK is not ready');this.currentQuality=value;},
    play(){throw new Error('Quality selection must not play');},
    pause(){throw new Error('Quality selection must not pause');},
    reload(){throw new Error('Quality selection must not reload');}};
  const controller=create(preference);controller.attach(sdk);
  return {controller,sdk,calls,refresh(paused=false){controller.refresh(paused);return calls.at(-1);}};
}
test('auto only requests advertised auto quality',()=>{
  const f=fixture();assert.equal(f.refresh(),'auto');
  const g=fixture('auto',['160p30','chunked']);g.refresh();assert.deepEqual(g.calls,[]);
});
test('exact 180p is selected from advertised SDK IDs',()=>{
  const f=fixture('180p',[{group:'180p30',name:'180p'},{group:'360p30',name:'360p'}]);
  assert.equal(f.refresh(),'180p30');
});
test('180p falls back to nearest available height rather than source',()=>{
  assert.equal(fixture('180p',['auto','360p30','720p30','chunked']).refresh(),'360p30');
  assert.equal(fixture('180p',['auto','160p30','360p30','chunked']).refresh(),'160p30');
});
test('equal resolution distance favors lower height',()=>{
  assert.equal(fixture('180p',['200p30','160p30']).refresh(),'160p30');
});
test('same height favors lower FPS independent of SDK ordering',()=>{
  assert.equal(fixture('720p',['720p60','720p30','1080p60']).refresh(),'720p30');
});
test('object display labels are recognized while only group IDs are requested',()=>{
  assert.equal(fixture('480p',[{group:'medium',name:'480p30'},{group:'high',name:'720p60'}]).refresh(),'medium');
});
test('numeric preference uses source-only stream when no transcoded heights exist',()=>{
  assert.equal(fixture('180p',['auto','chunked']).refresh(),'chunked');
});
test('unknown source height does not compete with available numeric choices',()=>{
  assert.equal(fixture('2160p',[{group:'chunked',name:'Source'}, {group:'720p30',name:'720p'}]).refresh(),'720p30');
});
test('source selects advertised chunked or source before numeric alternatives',()=>{
  assert.equal(fixture('source',['auto','1080p60','chunked']).refresh(),'chunked');
  assert.equal(fixture('source',['auto','source','1080p60']).refresh(),'source');
});
test('source falls back to highest numeric resolution when source is not advertised',()=>{
  assert.equal(fixture('source',['auto','720p30','1080p30','160p30']).refresh(),'1080p30');
});
test('empty variants defer and later refresh applies preference',()=>{
  const f=fixture('180p',[]);f.refresh();assert.deepEqual(f.calls,[]);
  f.sdk.qualities=['360p30'];assert.equal(f.refresh(),'360p30');
});
test('unchanged preference and variants do not spam setter',()=>{
  const f=fixture('180p');f.refresh();const count=f.calls.length;
  for(let i=0;i<5;i++){f.controller.setPreference('180p');f.refresh();}
  assert.equal(f.calls.length,count);
});
test('changed variants reconsider the nearest quality',()=>{
  const f=fixture('180p',['360p30']);assert.equal(f.refresh(),'360p30');
  f.sdk.qualities=['180p30','360p30'];assert.equal(f.refresh(),'180p30');
});
test('changed preference applies without playback or reload calls',()=>{
  const f=fixture('180p');f.refresh();f.controller.setPreference('720p');assert.equal(f.refresh(),'720p30');
  f.controller.setPreference('auto');assert.equal(f.refresh(),'auto');
});
test('invalid preference changes leave the prior preference intact',()=>{
  const f=fixture('180p');f.refresh();const count=f.calls.length;
  for(const invalid of ['',null,undefined,'180','999p','720P','javascript:bad']) {f.controller.setPreference(invalid);f.refresh();}
  assert.equal(f.calls.at(-1),'160p30');assert.equal(f.calls.length,count);
});
test('failed setter can retry on later refresh',()=>{
  const f=fixture('180p',[]);f.sdk.fail=true;f.sdk.qualities=['160p30'];f.refresh();
  const failed=f.calls.length;assert(failed>0);f.sdk.fail=false;assert.equal(f.refresh(),'160p30');
  assert.equal(f.calls.length,failed+1);f.refresh();assert.equal(f.calls.length,failed+1);
});
test('explicit paused refresh defers until resume',()=>{
  const f=fixture('180p',[]);f.sdk.qualities=['160p30'];f.refresh(true);assert.deepEqual(f.calls,[]);
  assert.equal(f.refresh(false),'160p30');
});
test('SDK pause state defers preference and applies it once resumed',()=>{
  const f=fixture('180p',[]);f.sdk.qualities=['160p30','720p30'];f.sdk.paused=true;
  f.controller.setPreference('720p');f.refresh();assert.deepEqual(f.calls,[]);
  f.sdk.paused=false;assert.equal(f.refresh(),'720p30');
});
test('all documented preferences select advertised values',()=>{
  for(const preference of ['auto','source','160p','180p','240p','360p','480p','720p','1080p','1440p','2160p']) {
    const f=fixture(preference,['auto','chunked','160p30','180p30','240p30','360p30','480p30','720p30','1080p30','1440p30','2160p30']);
    assert.equal(f.refresh(),preference==='source'?'chunked':preference==='auto'?'auto':`${preference}30`);
  }
});
test('already active desired quality does not call setter',()=>{
  const f=fixture('180p',[]);f.sdk.currentQuality='180p30';f.sdk.qualities=['180p30','360p30'];
  f.refresh();assert.deepEqual(f.calls,[]);
});
test('audio-only and unrecognized metadata are never requested as video',()=>{
  const f=fixture('180p',['audio_only',null,{}, {group:'bad id',name:'180p'}, '360p30']);
  assert.equal(f.refresh(),'360p30');
});
