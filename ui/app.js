import { playbackLabel, observedPlaying, secondsAgo } from './view-model.js';
const $ = id => document.getElementById(id);
let state, listKey = '', playersKey = '', dragging = false, refreshing = false, audioTimer, localError = null;
const native = () => window.__TAURI__?.core;
function showError(error) { localError = String(error); $('error-text').textContent = localError; $('error').hidden = false; }
async function act(action) {
  try {
    if (!native()) throw new Error('Open MPD Tabber as the native application. A browser-only preview cannot run the Rust controller.');
    await native().invoke('dispatch', { action });
    localError = null;
    await refresh();
    return true;
  } catch (error) { showError(error); return false; }
}
function button(text, label, action, className='icon-button') {
  const b = document.createElement('button'); b.type='button'; b.textContent=text; b.className=className;
  b.setAttribute('aria-label',label); b.title=label; b.addEventListener('click',()=>act(action)); return b;
}
function node(tag, text='', className='') {
  const e=document.createElement(tag); e.textContent=text; e.className=className; return e;
}
function favorites(s) {
  const key=JSON.stringify([s.favorites,s.players.map(p=>[p.login,p.closing]),s.settings.demo]);
  if (key===listKey || dragging) return;
  listKey=key; const assigned=new Set(s.players.filter(p=>!p.closing).map(p=>p.login));
  $('favorites').replaceChildren(...s.favorites.map((f,i)=>{
    const li=node('li','','favorite'); li.draggable=true; li.dataset.login=f.login; li.setAttribute('aria-disabled',String(!f.enabled));
    li.append(node('span',String(i+1).padStart(2,'0'),'rank'));
    const info=node('div','','channel-info'); info.append(node('strong',f.login));
    const sub=node('div','','channel-sub'); sub.append(node('span',f.presence==='live'?'● LIVE':f.presence.toUpperCase(),f.presence==='live'?'live':''));
    if (assigned.has(f.login)) sub.append(node('span','SELECTED','selected-label'));
    if (f.skipped) sub.append(node('span','SKIPPED'));
    if (f.open_error) { const e=node('span','OPEN FAILED'); e.title=f.open_error; sub.append(e); }
    info.append(sub); li.append(info);
    const controls=node('div','','row-controls');
    if (s.settings.demo) controls.append(button(f.demo_live?'Go offline':'Go live',`Simulate ${f.login} ${f.demo_live?'offline':'live'}`,{type:'demo_live',login:f.login,live:!f.demo_live},'toggle-live'));
    const enabled=document.createElement('input'); enabled.type='checkbox'; enabled.checked=f.enabled; enabled.title=`Enable ${f.login}`; enabled.setAttribute('aria-label',enabled.title); enabled.addEventListener('change',()=>act({type:'enable',login:f.login,enabled:enabled.checked})); controls.append(enabled);
    const up=button('↑',`Move ${f.login} up`,{type:'move',login:f.login,position:i-1}); up.disabled=i===0;
    const down=button('↓',`Move ${f.login} down`,{type:'move',login:f.login,position:i+1}); down.disabled=i===s.favorites.length-1;
    controls.append(up,down);
    if (f.skipped) controls.append(button('Undo',`Undo skip for ${f.login}`,{type:'undo_skip',login:f.login}));
    if (f.open_error) controls.append(button('Retry',`Retry ${f.login}`,{type:'retry',login:f.login}));
    controls.append(button('×',`Remove ${f.login}`,{type:'remove',login:f.login})); li.append(controls);
    li.addEventListener('dragstart',e=>{dragging=true;e.dataTransfer.setData('text/plain',f.login);e.dataTransfer.effectAllowed='move';});
    li.addEventListener('dragover',e=>{e.preventDefault();li.classList.add('drag-target');});
    li.addEventListener('dragleave',()=>li.classList.remove('drag-target'));
    li.addEventListener('drop',e=>{e.preventDefault();dragging=false;listKey='';li.classList.remove('drag-target');const login=e.dataTransfer.getData('text/plain');if(s.favorites.some(x=>x.login===login))act({type:'move',login,position:i});});
    li.addEventListener('dragend',()=>{dragging=false;listKey='';if(state)render(state);}); return li;
  }));
}
function players(s) {
  const key=JSON.stringify([s.players,s.settings.demo]); if(key===playersKey)return; playersKey=key;
  $('players').replaceChildren(...s.players.map(p=>{
    const card=node('article','','player-card'); card.append(node('strong',p.login));
    card.append(node('span',playbackLabel(p,s.settings.demo),'player-state'));
    const volume=p.volume==null?'unknown':`${Math.round(p.volume*100)}%`;
    const muted=p.muted==null?'unknown':p.muted?'yes':'no';
    const visibility=p.visible==null?'unknown':p.visible?'visible':'hidden';
    card.append(node('div',`Session ${p.session} · Document ${visibility}\nObserved volume ${volume} · Muted ${muted}`,'player-meta'));
    const actions=node('div','','button-row'); actions.append(button('Focus player',`Focus ${p.login}`,{type:'focus',login:p.login},''),button('Skip',`Skip ${p.login}`,{type:'skip',login:p.login},''),button('Retry',`Retry ${p.login}`,{type:'retry',login:p.login},'')); card.append(actions); return card;
  }));
}
function render(s) {
  state=s;
  $('run-status').textContent=s.mode==='running'?'Monitoring':s.mode==='paused'?'Automation paused':'Stopped';
  $('run-status').className=`pill ${s.mode}`;
  $('start').textContent=s.mode==='paused'?'Resume':'Start'; $('start').disabled=s.mode==='running';
  $('pause').disabled=s.mode!=='running'; $('stop').disabled=s.mode==='stopped'&&s.players.length===0;
  $('refresh').disabled=s.polling||s.mode==='stopped';
  $('session-count').replaceChildren(document.createTextNode(`${s.players.length} `),node('small',`/ ${s.settings.limit}`));
  $('playing-count').textContent=String(observedPlaying(s.players));
  $('playing-heading').textContent=s.settings.demo?'SIMULATED PLAYBACK':'OBSERVED PLAYBACK';
  const blocked=s.players.filter(p=>p.state==='blocked'&&!p.closing).length;
  $('playing-detail').textContent=blocked?`${blocked} player${blocked===1?'':'s'} need a click`:'Based on recent player telemetry, not viewer credit';
  $('monitor-heading').textContent=s.mode==='stopped'?'Not running':s.settings.demo?'Demo source':s.polling?'Checking…':s.mode==='paused'?'Selection paused':'Every 30s';
  $('monitor-detail').textContent=secondsAgo(s.last_check_seconds);
  $('favorite-count').textContent=String(s.favorites.length);
  $('empty-favorites').hidden=s.favorites.length>0; $('load-demo').hidden=!s.settings.demo;
  $('demo-note').textContent=s.settings.demo?'Demo switches simulate live status. No Twitch streams or viewers are created.':'Unknown or stale channels are not opened. Existing players survive temporary API failures.';
  if(document.activeElement!==$('source'))$('source').value=s.settings.demo?'demo':'twitch';
  if(document.activeElement!==$('limit'))$('limit').value=s.settings.limit;
  if(document.activeElement!==$('volume')){$('volume').value=s.settings.volume;$('volume-value').textContent=`${s.settings.volume}%`;}
  if(document.activeElement!==$('muted'))$('muted').checked=s.settings.muted;
  if(document.activeElement!==$('client-id'))$('client-id').value=s.settings.client_id;
  $('auth-status').textContent=s.connected_as?`Monitoring as ${s.connected_as}`:s.auth_pending?'Waiting for authorization…':'Not connected';
  $('viewer-signin').disabled=s.settings.demo;
  $('connect').disabled=s.auth_pending; $('disconnect').disabled=!s.connected_as&&!s.auth_pending;
  $('disconnect').textContent=s.auth_pending?'Cancel':'Disconnect';
  $('device-auth').hidden=!s.user_code; $('device-code').textContent=s.user_code??'';
  $('empty-players').hidden=s.players.length>0;
  $('player-origin').textContent=s.player_origin; $('events').textContent=s.events.join('\n');
  const error=s.error??localError;
  $('error').hidden=!error; if(error)$('error-text').textContent=error;
  favorites(s);players(s);
}
async function refresh() {
  if(refreshing||!native())return;
  refreshing=true;try{render(await native().invoke('get_state'));}catch(e){showError(e);}finally{refreshing=false;}
}
$('add-form').addEventListener('submit',e=>{e.preventDefault();const input=$('channel').value;act({type:'add',input}).then(ok=>{if(ok)$('channel').value='';});});
$('start').addEventListener('click',()=>act({type:'start'}));
$('pause').addEventListener('click',()=>act({type:'pause'}));
$('stop').addEventListener('click',()=>act({type:'stop'}));
$('refresh').addEventListener('click',()=>act({type:'refresh'}));
$('load-demo').addEventListener('click',()=>act({type:'load_demo'}));
$('source').addEventListener('change',()=>act({type:'set_demo',demo:$('source').value==='demo'}));
$('limit').addEventListener('change',()=>{const limit=Number($('limit').value);if(Number.isSafeInteger(limit)&&limit>=1&&limit<=1000)act({type:'set_limit',limit});else showError('Set a whole-number limit between 1 and 1000.');});
function changeAudio(){clearTimeout(audioTimer);$('volume-value').textContent=`${$('volume').value}%`;audioTimer=setTimeout(()=>act({type:'set_audio',volume:Number($('volume').value),muted:$('muted').checked}),120);}
$('volume').addEventListener('input',changeAudio);$('muted').addEventListener('change',changeAudio);
$('connect-form').addEventListener('submit',e=>{e.preventDefault();act({type:'connect',client_id:$('client-id').value});});
$('disconnect').addEventListener('click',()=>act({type:'disconnect'}));
$('viewer-signin').addEventListener('click',()=>act({type:'open_viewer_login'}));
$('open-auth').addEventListener('click',()=>act({type:'open_auth'}));
$('dismiss-error').addEventListener('click',()=>{localError=null;$('error').hidden=true;act({type:'clear_error'});});
if(!native())showError('Browser-only view: launch with cargo run -p mpd-tabber --features custom-protocol to use the Rust application.');
refresh();setInterval(refresh,750);
