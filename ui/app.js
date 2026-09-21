import { playbackLabel, observedPlaying, secondsAgo, viewerCountLabel } from './view-model.js';
const $ = id => document.getElementById(id);
let state, listKey = '', playersKey = '', dragging = null, moving = false, rankRecovery = null, refreshTask = null, audioTimer, localError = null;
const native = () => window.__TAURI__?.core;
function showError(error) { localError = String(error); $('error-text').textContent = localError; $('error').hidden = false; }
async function act(action) {
  try {
    if (!native()) throw new Error('Open MPD Tabber as the native application. A browser-only preview cannot run the Rust controller.');
    await native().invoke('dispatch', { action });
    localError = null;
    return await refresh(true);
  } catch (error) { showError(error); return false; }
}
function button(text, label, action, className='icon-button') {
  const b = document.createElement('button'); b.type='button'; b.textContent=text; b.className=className;
  b.setAttribute('aria-label',label); b.title=label; b.addEventListener('click',()=>typeof action==='function'?action():act(action)); return b;
}
function node(tag, text='', className='') {
  const e=document.createElement(tag); e.textContent=text; e.className=className; return e;
}
const rankStatus = text => { $('rank-status').textContent=text; };
const favoriteOrder = () => state?.favorites.map(f=>f.login).join('\0');
function clearDropMarker() {
  $('favorites').querySelectorAll('.drop-before,.drop-after').forEach(row=>row.classList.remove('drop-before','drop-after'));
}
function endDrag() {
  clearDropMarker();
  $('favorites').querySelectorAll('.drag-source').forEach(row=>row.classList.remove('drag-source'));
  dragging=null; listKey='';
}
function cancelDrag(message='Reordering cancelled.') {
  if(!dragging)return;
  endDrag(); rankStatus(message); if(state)favorites(state);
}
function finishRankMove(login, focusDirection) {
  moving=false; rankRecovery=null; listKey=''; $('favorites').removeAttribute('aria-busy');
  if(state)favorites(state);
  const index=state.favorites.findIndex(f=>f.login===login);
  rankStatus(index<0?'Channel list refreshed.':`Moved ${login} to rank ${index+1}.`);
  if(focusDirection) {
    const row=Array.from($('favorites').children).find(row=>row.dataset.login===login);
    const control=row?.querySelector(`[data-move="${focusDirection}"]`);
    if(control&&!control.disabled)control.focus();
    else row?.querySelector('[data-move]:not(:disabled)')?.focus();
  }
}
async function moveFavorite(login, position, focusDirection) {
  if(moving)return;
  const current=state.favorites.findIndex(f=>f.login===login);
  if(current<0)return;
  position=Math.max(0,Math.min(position,state.favorites.length-1));
  if(position===current){rankStatus(`${login} stays at rank ${current+1}.`);if(state)favorites(state);return;}
  moving=true; $('favorites').setAttribute('aria-busy','true'); rankStatus(`Saving rank for ${login}…`);
  // Keep the current DOM in place until Rust returns the saved ordering.
  try {
    if(!native())throw new Error('Open the native application to change channel ranks.');
    await native().invoke('dispatch',{action:{type:'move',login,position}});
    localError=null;
  } catch(error) {
    showError(error); moving=false; listKey=''; $('favorites').removeAttribute('aria-busy');
    if(state)favorites(state);
    rankStatus(`Could not save the rank for ${login}. Check the error and try again.`); return;
  }
  if(await refresh(true))finishRankMove(login,focusDirection);
  else {
    // Rust saved the move. Do not submit another rank based on our stale view.
    rankRecovery={login,focusDirection};
    rankStatus(`Saved rank for ${login}. Waiting to refresh the channel order before reordering again.`);
  }
}
function dropPosition(event) {
  if(!dragging||moving)return null;
  const rows=Array.from($('favorites').children);
  let row=event.target.closest?.('.favorite');
  if(!row||row.parentElement!==$('favorites')) {
    row=rows.at(-1);
    if(!row)return null;
  }
  const rect=row.getBoundingClientRect();
  const before=event.clientY<rect.top+rect.height/2;
  const source=rows.findIndex(row=>row.dataset.login===dragging.login);
  const slot=rows.indexOf(row)+(before?0:1);
  return {row,before,position:slot-(source<slot?1:0)};
}
$('favorites').addEventListener('dragover',event=>{
  const target=dropPosition(event); if(!target)return;
  event.preventDefault(); event.dataTransfer.dropEffect='move'; clearDropMarker();
  target.row.classList.add(target.before?'drop-before':'drop-after');
});
$('favorites').addEventListener('dragleave',event=>{
  if(!$('favorites').contains(event.relatedTarget))clearDropMarker();
});
$('favorites').addEventListener('drop',event=>{
  event.preventDefault();
  const target=dropPosition(event); if(!target)return;
  if(event.dataTransfer.getData('application/x-mpd-channel')!==dragging.login) {
    cancelDrag(); return;
  }
  if(favoriteOrder()!==dragging.order) {
    cancelDrag('Channel order changed during the drag. Please try again.'); return;
  }
  const login=dragging.login; endDrag();
  void moveFavorite(login,target.position);
});
// File/URL drops must never navigate the manager away from its local UI.
document.addEventListener('dragover',event=>{
  event.preventDefault();
  if(!dragging&&event.dataTransfer)event.dataTransfer.dropEffect='none';
});
document.addEventListener('drop',event=>{event.preventDefault();cancelDrag();});
document.addEventListener('keydown',event=>{if(event.key==='Escape')cancelDrag();});
document.addEventListener('visibilitychange',()=>{if(document.hidden)cancelDrag();});
function favorites(s) {
  const key=JSON.stringify([s.favorites,s.players.map(p=>[p.login,p.closing]),s.settings.demo]);
  if (key===listKey || dragging || moving) return;
  listKey=key; const assigned=new Set(s.players.filter(p=>!p.closing).map(p=>p.login));
  $('favorites').replaceChildren(...s.favorites.map((f,i)=>{
    const li=node('li','','favorite'); li.draggable=true; li.dataset.login=f.login; li.setAttribute('aria-disabled',String(!f.enabled));
    const rank=node('span','','rank');
    const grip=node('span','⠿','drag-grip'); grip.setAttribute('aria-hidden','true');
    rank.append(grip,node('span',String(i+1).padStart(2,'0'))); li.append(rank);
    li.title=`Drag ${f.login} to change rank, or use its up/down arrows`;
    const info=node('div','','channel-info'); info.append(node('strong',f.login));
    const sub=node('div','','channel-sub'); sub.append(node('span',f.presence==='live'?'● LIVE':f.presence.toUpperCase(),f.presence==='live'?'live':''));
    const audience = s.settings.demo ? '' : viewerCountLabel(f.viewer_count);
    if (audience) sub.append(node('span',audience,'viewer-count'));
    if (assigned.has(f.login)) sub.append(node('span','SELECTED','selected-label'));
    if (f.skipped) sub.append(node('span','SKIPPED'));
    if (f.open_error) { const e=node('span','OPEN FAILED'); e.title=f.open_error; sub.append(e); }
    info.append(sub); li.append(info);
    const controls=node('div','','row-controls');
    if (s.settings.demo) controls.append(button(f.demo_live?'Go offline':'Go live',`Simulate ${f.login} ${f.demo_live?'offline':'live'}`,{type:'demo_live',login:f.login,live:!f.demo_live},'toggle-live'));
    const enabled=document.createElement('input'); enabled.type='checkbox'; enabled.checked=f.enabled; enabled.title=`Enable ${f.login}`; enabled.setAttribute('aria-label',enabled.title); enabled.addEventListener('change',()=>act({type:'enable',login:f.login,enabled:enabled.checked})); controls.append(enabled);
    const up=button('↑',`Move ${f.login} up`,()=>moveFavorite(f.login,state.favorites.findIndex(x=>x.login===f.login)-1,'up')); up.disabled=i===0;
    const down=button('↓',`Move ${f.login} down`,()=>moveFavorite(f.login,state.favorites.findIndex(x=>x.login===f.login)+1,'down')); down.disabled=i===s.favorites.length-1;
    up.dataset.move='up'; down.dataset.move='down'; controls.append(up,down);
    if (f.skipped) controls.append(button('Undo',`Undo skip for ${f.login}`,{type:'undo_skip',login:f.login}));
    if (f.open_error) controls.append(button('Retry',`Retry ${f.login}`,{type:'retry',login:f.login}));
    controls.append(button('×',`Remove ${f.login}`,{type:'remove',login:f.login})); li.append(controls);
    li.addEventListener('pointerdown',event=>{
      li.draggable=!moving&&!event.target.closest?.('button,input,select,a');
    });
    li.addEventListener('dragstart',event=>{
      if(moving||event.target.closest?.('button,input,select,a')){event.preventDefault();return;}
      dragging={login:f.login,order:favoriteOrder()}; li.classList.add('drag-source');
      event.dataTransfer.setData('application/x-mpd-channel',f.login);
      event.dataTransfer.setData('text/plain',f.login); event.dataTransfer.effectAllowed='move';
      rankStatus(`Moving ${f.login}. Drop at the line to set its rank, or press Escape to cancel.`);
    });
    li.addEventListener('dragend',()=>cancelDrag()); return li;
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
  if(document.activeElement!==$('quality'))$('quality').value=s.settings.preferred_quality??'auto';
  if(document.activeElement!==$('limit'))$('limit').value=s.settings.limit;
  if(document.activeElement!==$('volume')){$('volume').value=s.settings.volume;$('volume-value').textContent=`${s.settings.volume}%`;}
  if(document.activeElement!==$('muted'))$('muted').checked=s.settings.muted;
  $('auth-status').textContent=s.connected_as?`Monitoring as ${s.connected_as}`:s.auth_pending?'Waiting for authorization…':'Not connected';
  $('connect').disabled=s.settings.demo||(s.auth_pending&&!s.user_code);
  $('connect').textContent=s.auth_pending?(s.user_code?'Continue in Twitch':'Opening Twitch…'):s.connected_as?'Reconnect Twitch':'Connect Twitch';
  $('disconnect').disabled=!s.connected_as&&!s.auth_pending;
  $('disconnect').textContent=s.auth_pending?'Cancel':'Disconnect';
  $('device-auth').hidden=!s.user_code; $('device-code').textContent=s.user_code??'';
  $('empty-players').hidden=s.players.length>0;
  $('player-origin').textContent=s.player_origin; $('events').textContent=s.events.join('\n');
  const error=s.error??localError;
  $('error').hidden=!error; if(error)$('error-text').textContent=error;
  favorites(s);players(s);
}
async function refresh(fresh=false) {
  if(!native())return false;
  if(refreshTask) {
    const pending=refreshTask, ok=await pending;
    if(!fresh)return ok;
    if(refreshTask===pending)refreshTask=null;
  }
  if(refreshTask)return refresh(fresh);
  const task=(async()=>{
    try {
      const snapshot=await native().invoke('get_state'); render(snapshot);
      if(rankRecovery) {
        localError=null; finishRankMove(rankRecovery.login,rankRecovery.focusDirection); render(snapshot);
      }
      return true;
    }
    catch(error){showError(error);return false;}
  })();
  refreshTask=task;
  try{return await task;}finally{if(refreshTask===task)refreshTask=null;}
}
$('add-form').addEventListener('submit',e=>{e.preventDefault();const input=$('channel').value;act({type:'add',input}).then(ok=>{if(ok)$('channel').value='';});});
$('start').addEventListener('click',()=>act({type:'start'}));
$('pause').addEventListener('click',()=>act({type:'pause'}));
$('stop').addEventListener('click',()=>act({type:'stop'}));
$('refresh').addEventListener('click',()=>act({type:'refresh'}));
$('load-demo').addEventListener('click',()=>act({type:'load_demo'}));
$('source').addEventListener('change',()=>act({type:'set_demo',demo:$('source').value==='demo'}));
$('quality').addEventListener('change',()=>act({type:'set_quality',quality:$('quality').value}));
$('limit').addEventListener('change',()=>{const limit=Number($('limit').value);if(Number.isSafeInteger(limit)&&limit>=1&&limit<=1000)act({type:'set_limit',limit});else showError('Set a whole-number limit between 1 and 1000.');});
function changeAudio(){clearTimeout(audioTimer);$('volume-value').textContent=`${$('volume').value}%`;audioTimer=setTimeout(()=>act({type:'set_audio',volume:Number($('volume').value),muted:$('muted').checked}),120);}
$('volume').addEventListener('input',changeAudio);$('muted').addEventListener('change',changeAudio);
$('connect').addEventListener('click',()=>act({type:'connect'}));
$('disconnect').addEventListener('click',()=>act({type:'disconnect'}));
$('dismiss-error').addEventListener('click',()=>{localError=null;$('error').hidden=true;act({type:'clear_error'});});
if(!native())showError('Browser-only view: launch with cargo run -p mpd-tabber --features custom-protocol to use the Rust application.');
refresh();setInterval(refresh,750);
