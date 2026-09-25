const $ = id => document.getElementById(id);
let bridge;
let state;
let activeChat;
let audioTimer;

const hues = [188, 270, 334, 36, 115, 218];
const chatLines = [
  ['pixelpilot', 'The four-up layout feels surprisingly readable.'],
  ['maplebyte', 'Audio focus makes sense on a phone.'],
  ['orbitcat', 'This transcript is intentionally simulated.'],
  ['lowlatency', 'Tablet landscape should be the real-video target.'],
];

async function resolveBridge() {
  if (window.__TAURI__?.core) return window.__TAURI__.core;
  if (new URLSearchParams(location.search).get('preview') === '1') {
    return (await import('./preview-bridge.js')).previewBridge;
  }
  throw new Error('Launch the Tauri mobile POC, or append ?preview=1 for the browser-only fixture.');
}

function element(tag, className, text) {
  const value = document.createElement(tag);
  if (className) value.className = className;
  if (text != null) value.textContent = text;
  return value;
}

function empty(message) {
  const value = element('div', 'empty-grid');
  value.append(element('span', '', message));
  return value;
}

function renderGrid(snapshot) {
  const grid = $('stream-grid');
  grid.replaceChildren();
  if (!snapshot.sessions.length) {
    grid.append(empty(snapshot.mode === 'stopped' ? 'Press Start to open the four simulated assignments.' : 'No eligible live favorites.'));
    return;
  }
  snapshot.sessions.forEach((session, index) => {
    const tile = element('article', 'stream-tile');
    tile.dataset.login = session.login;
    tile.style.setProperty('--hue', hues[index % hues.length]);
    tile.append(element('div', 'stream-art'));
    const top = element('div', 'stream-top');
    top.append(element('span', 'badge', 'LIVE'), element('span', 'session-id', `S${session.session}`));
    const bottom = element('div', 'stream-bottom');
    bottom.append(element('strong', 'stream-login', session.login));
    const sound = element('button', 'sound', snapshot.muted ? '⌁' : '◖');
    sound.type = 'button';
    sound.ariaLabel = `${snapshot.muted ? 'Unmute' : 'Mute'} ${session.login}`;
    sound.addEventListener('click', () => act({ type: 'set_audio', volume: snapshot.volume, muted: !snapshot.muted }));
    bottom.append(sound);
    tile.append(top, bottom);
    grid.append(tile);
  });
}

function renderAudio(snapshot) {
  const list = $('audio-sessions');
  list.replaceChildren();
  if (!snapshot.sessions.length) {
    list.append(empty('Start monitoring to populate audio sessions.'));
    activeChat = undefined;
  } else {
    if (!snapshot.sessions.some(session => session.login === activeChat)) activeChat = snapshot.sessions[0].login;
    snapshot.sessions.forEach(session => {
      const button = element('button', `audio-card${session.login === activeChat ? ' active' : ''}`);
      button.type = 'button';
      const icon = element('span', 'audio-icon', snapshot.muted ? '⌁' : '◖');
      const copy = element('span');
      copy.append(element('strong', '', session.login), element('span', '', `${snapshot.muted ? 'Muted' : `${snapshot.volume}%`} · tap for chat`));
      button.append(icon, copy);
      button.addEventListener('click', () => { activeChat = session.login; renderAudio(snapshot); });
      list.append(button);
    });
  }
  $('chat-channel').textContent = activeChat ? `#${activeChat}` : 'Choose a channel';
  const messages = $('chat-messages');
  messages.replaceChildren();
  if (!activeChat) {
    messages.append(element('p', 'chat-message', 'No active chat.'));
  } else {
    chatLines.forEach(([name, message], index) => {
      const row = element('p', 'chat-message');
      row.style.setProperty('--hue', hues[index % hues.length]);
      row.append(element('strong', '', name), document.createTextNode(message));
      messages.append(row);
    });
  }
}

function renderFavorites(snapshot) {
  const list = $('favorites');
  list.replaceChildren();
  snapshot.favorites.forEach((favorite, index) => {
    const row = element('li', 'favorite');
    row.append(element('span', 'favorite-rank', String(index + 1).padStart(2, '0')));
    const name = element('span', 'favorite-name');
    name.append(element('strong', '', favorite.login), element('span', '', `${favorite.live ? 'simulated live' : 'offline'}${favorite.selected ? ' · selected' : ''}`));
    row.append(name);
    const controls = element('span', 'favorite-controls');
    const up = element('button', '', '↑');
    up.type = 'button'; up.disabled = index === 0; up.ariaLabel = `Move ${favorite.login} up`;
    up.addEventListener('click', () => act({ type: 'move', login: favorite.login, position: index - 1 }));
    const live = element('button', '', favorite.live ? 'Live' : 'Off');
    live.type = 'button'; live.ariaLabel = `Mark ${favorite.login} ${favorite.live ? 'offline' : 'live'}`;
    live.addEventListener('click', () => act({ type: 'demo_live', login: favorite.login, live: !favorite.live }));
    const remove = element('button', '', '×');
    remove.type = 'button'; remove.ariaLabel = `Remove ${favorite.login}`;
    remove.addEventListener('click', () => act({ type: 'remove', login: favorite.login }));
    controls.append(up, live, remove); row.append(controls); list.append(row);
  });
}

function render(snapshot) {
  state = snapshot;
  $('run-status').textContent = snapshot.mode[0].toUpperCase() + snapshot.mode.slice(1);
  $('run-status').className = `status ${snapshot.mode}`;
  $('session-count').textContent = `${snapshot.sessions.length} / ${snapshot.limit}`;
  $('show-grid').classList.toggle('selected', snapshot.display === 'grid');
  $('show-audio').classList.toggle('selected', snapshot.display === 'audio_chat');
  $('show-grid').setAttribute('aria-pressed', String(snapshot.display === 'grid'));
  $('show-audio').setAttribute('aria-pressed', String(snapshot.display === 'audio_chat'));
  $('grid-view').hidden = snapshot.display !== 'grid';
  $('audio-view').hidden = snapshot.display !== 'audio_chat';
  $('start').disabled = snapshot.mode === 'running';
  $('start').textContent = snapshot.mode === 'paused' ? 'Resume' : 'Start';
  $('pause').disabled = snapshot.mode !== 'running';
  $('stop').disabled = snapshot.mode === 'stopped' && snapshot.sessions.length === 0;
  $('limit').value = String(snapshot.limit);
  $('volume').value = String(snapshot.volume);
  $('volume-value').textContent = `${snapshot.volume}%`;
  $('muted').checked = snapshot.muted;
  $('events').textContent = snapshot.events.join('\n');
  renderGrid(snapshot);
  renderAudio(snapshot);
  renderFavorites(snapshot);
}

function showError(error) {
  const message = $('error');
  message.textContent = String(error);
  message.hidden = false;
  clearTimeout(showError.timer);
  showError.timer = setTimeout(() => { message.hidden = true; }, 5000);
}

async function act(action) {
  try {
    render(await bridge.invoke('dispatch', { action }));
  } catch (error) {
    showError(error);
  }
}

document.querySelectorAll('[data-display]').forEach(button => button.addEventListener('click', () => act({ type: 'set_display', display: button.dataset.display })));
$('start').addEventListener('click', () => act({ type: 'start' }));
$('pause').addEventListener('click', () => act({ type: 'pause' }));
$('stop').addEventListener('click', () => act({ type: 'stop' }));
$('load-demo').addEventListener('click', () => act({ type: 'load_demo' }));
$('limit').addEventListener('change', () => act({ type: 'set_limit', limit: Number($('limit').value) }));
$('volume').addEventListener('input', () => {
  $('volume-value').textContent = `${$('volume').value}%`;
  clearTimeout(audioTimer);
  audioTimer = setTimeout(() => act({ type: 'set_audio', volume: Number($('volume').value), muted: $('muted').checked }), 100);
});
$('muted').addEventListener('change', () => act({ type: 'set_audio', volume: Number($('volume').value), muted: $('muted').checked }));
$('add-form').addEventListener('submit', event => {
  event.preventDefault();
  const input = $('channel').value;
  act({ type: 'add', input }).then(() => { $('channel').value = ''; });
});

try {
  bridge = await resolveBridge();
  render(await bridge.invoke('get_state'));
} catch (error) {
  showError(error);
}
