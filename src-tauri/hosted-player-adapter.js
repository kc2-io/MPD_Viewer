function installHostedPlayerAdapter(config, createQualityController) {
  'use strict';
  // Native-only compatibility with the deployed, unversioned query-string host.
  // The preserved hosted source is not replaced or granted manager permissions.
  if (window.top !== window || location.origin !== config.origin || location.pathname !== config.path) return;
  const query = new URLSearchParams(location.search);
  if (query.getAll('channel').length !== 1 || query.get('channel') !== config.channel) return;
  let volume = config.volume, muted = config.muted, ready = false, state = 'loading';
  let player = null, notice = null;
  const quality = typeof createQualityController === 'function' ? createQualityController(config.quality) : null;
  window.mpdSetQuality = value => quality?.setPreference(value);
  const states = {READY: 'ready', PLAYING: 'playing', PAUSE: 'paused',
    PLAYBACK_BLOCKED: 'blocked', OFFLINE: 'offline', ENDED: 'ended'};
  const commands = new Set(['add', 'remove', 'setChannels', 'setActive', 'setVolume', 'pauseInactive', 'ping']);

  function report() {
    quality?.refresh(state === 'paused' || state === 'blocked' || state === 'offline' || state === 'ended');
    const invoke = window.__TAURI__?.core?.invoke;
    if (!invoke) return;
    let actualVolume = null, actualMuted = null;
    if (ready && player) {
      try {
        const v = player.getVolume(), m = player.getMuted();
        if (Number.isFinite(v) && v >= 0 && v <= 1) actualVolume = v;
        if (typeof m === 'boolean') actualMuted = m;
      } catch { /* Observations remain unknown until the SDK is ready. */ }
    }
    try {
      Promise.resolve(invoke('player_report', {report: {session: config.session, state,
        visible: document.visibilityState === 'visible', volume: actualVolume, muted: actualMuted}})).catch(() => {});
    } catch { /* A denied/unavailable telemetry bridge must not disrupt playback. */ }
  }

  function showError(message) {
    state = 'error';
    if (document.body && !notice) {
      notice = document.createElement('div');
      notice.setAttribute('role', 'alert');
      notice.style.cssText = 'position:fixed;inset:16px 16px auto;z-index:2147483647;background:#241c30;color:#fff;padding:16px;font:14px system-ui;border:1px solid #a48ac7';
      notice.textContent = message;
      document.body.appendChild(notice);
    }
    report();
  }

  function applyAudio() {
    if (!ready || !player) return;
    try {
      // Never call legacy setVolume/applyState: those can resume manual pauses.
      player.setVolume(volume / 100);
      player.setMuted(muted);
    } catch { showError('Could not apply player audio. Close this viewer and use Retry in MPD Viewer.'); }
  }

  window.mpdSetAudio = (nextVolume, nextMuted) => {
    if (!Number.isFinite(nextVolume) || nextVolume < 0 || nextVolume > 100 || typeof nextMuted !== 'boolean') return;
    volume = nextVolume; muted = nextMuted;
    applyAudio(); report();
  };

  window.addEventListener('message', event => {
    let message = event.data;
    let oversized = false;
    if (typeof message === 'string') {
      oversized = message.length > 4096;
      try { message = JSON.parse(message); } catch { return; }
    }
    if (!message || typeof message !== 'object') return;
    // Native code owns assignments and audio. The legacy page accepts arbitrary
    // postMessage commands; block that command vocabulary before its listener.
    // Other messages (including Twitch SDK iframe traffic) remain untouched.
    if (commands.has(message.type)) { event.stopImmediatePropagation(); return; }
    if (oversized) return;
    if (event.origin !== config.origin || event.source !== window) return;
    if (message.type !== 'event' || message.channel !== config.channel) return;
    if (!Object.hasOwn(states, message.event)) return; // ONLINE is not playback.
    if (message.event === 'READY') {
      const entries = window.players;
      const entry = entries && Object.hasOwn(entries, config.channel) ? entries[config.channel] : null;
      if (!entry?.player || Object.keys(entries).length !== 1) {
        showError('The hosted player is incompatible with this MPD Viewer build. Check the configured player page.');
        return;
      }
      player = entry.player;
      ready = true;
      state = 'ready';
      if (notice) { notice.remove(); notice = null; }
      applyAudio();
      quality?.attach(player);
    } else {
      state = states[message.event];
    }
    report();
  }, true);

  document.addEventListener('visibilitychange', report);
  setInterval(report, 5000); // Advisory observations only; never play/reload.
  setTimeout(() => {
    if (!ready) showError('Twitch did not initialize. Check your connection and the hosted player page, then close this viewer and use Retry.');
  }, 30000);
}
