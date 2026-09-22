function mpdInstallChat(config) {
  'use strict';
  if (window.top !== window || !/^[a-z0-9_]{1,25}$/.test(config.channel)) return;
  if (config.mode === 'hosted' && (location.origin !== config.origin || location.pathname !== config.path ||
      new URLSearchParams(location.search).getAll('channel').length !== 1 ||
      new URLSearchParams(location.search).get('channel') !== config.channel)) return;
  function install() {
    if (document.getElementById('mpd-chat-panel')) return;
    const hosted = config.mode === 'hosted';
    const layout = hosted ? document.body : document.getElementById('mpd-viewer-layout');
    const video = document.getElementById(hosted ? 'grid' : 'mpd-viewer-video');
    if (!layout || !video || video.parentElement !== layout) return;
    if (hosted) {
      const style = document.createElement('style');
      style.textContent = config.css;
      document.head.append(style);
      document.documentElement.classList.add('mpd-chat-document');
    }
    layout.classList.add('mpd-chat-layout');
    video.classList.add('mpd-chat-video');
    const toolbar = document.createElement('div');
    toolbar.className = 'mpd-chat-toolbar';
    const label = document.createElement('strong');
    label.textContent = `${config.channel} · ${config.demo ? 'Simulated chat' : 'Twitch chat'}`;
    const toggle = document.createElement('button');
    toggle.type = 'button';
    toggle.id = 'mpd-chat-toggle';
    toggle.textContent = 'Hide chat';
    toggle.setAttribute('aria-expanded', 'true');
    toggle.setAttribute('aria-controls', 'mpd-chat-panel');
    toolbar.append(label, toggle);
    const panel = document.createElement('aside');
    panel.id = 'mpd-chat-panel';
    panel.setAttribute('aria-label', `${config.channel} chat`);
    toggle.addEventListener('click', () => {
      panel.hidden = !panel.hidden;
      layout.dataset.chatCollapsed = String(panel.hidden);
      toggle.textContent = panel.hidden ? 'Show chat' : 'Hide chat';
      toggle.setAttribute('aria-expanded', String(!panel.hidden));
    });
    if (config.demo) {
      const note = document.createElement('p');
      note.className = 'mpd-chat-note';
      note.textContent = 'SIMULATED · No messages are sent to Twitch.';
      const messages = document.createElement('div');
      messages.className = 'mpd-chat-demo';
      for (const text of ['Demo viewer: Welcome to the stream!', 'Demo moderator: Chat stays beside this channel.', 'Demo viewer: Hide chat to give the video more room.']) {
        const message = document.createElement('p');
        message.textContent = text;
        messages.append(message);
      }
      panel.append(note, messages);
    } else {
      const note = document.createElement('p');
      note.className = 'mpd-chat-note';
      note.textContent = 'Official Twitch chat. For account access, use Connect Twitch in the MPD Viewer manager.';
      const status = document.createElement('p');
      status.className = 'mpd-chat-note';
      status.id = 'mpd-chat-status';
      status.setAttribute('role', 'status');
      status.textContent = 'Loading Twitch chat…';
      const frame = document.createElement('iframe');
      frame.title = `${config.channel} Twitch chat`;
      // The OS is the only theme authority. The chat frame does not inherit
      // this page's prefers-color-scheme, so the theme must cross into its URL.
      let theme = false;
      let media = null;
      try {
        media = window.matchMedia('(prefers-color-scheme: dark)');
        theme = media.matches === true;
      } catch { media = null; theme = false; }
      const themedURL = () => {
        const url = new URL(`https://www.twitch.tv/embed/${config.channel}/chat`);
        url.searchParams.set('parent', location.hostname);
        if (theme) url.search += '&darkpopout'; // the frozen exact dark shape
        return url.href;
      };
      frame.referrerPolicy = 'strict-origin-when-cross-origin';
      // Loading the frame is not proof of a signed-in session or working posting.
      // Every navigation (initial install or OS theme change) restarts loading
      // feedback and the 20-second timeout so a stalled frame cannot leave the
      // status hidden and a previous error cannot survive into a retry.
      let loading = null;
      const showStatus = (text) => { status.hidden = false; status.textContent = text; };
      const navigate = (url) => {
        if (loading) { clearTimeout(loading); loading = null; }
        showStatus('Loading Twitch chat…');
        loading = setTimeout(() => {
          showStatus('Chat is taking longer to load. Video playback is independent of chat.');
        }, 20000);
        frame.src = url; // exactly one assignment per navigation
      };
      navigate(themedURL());
      frame.addEventListener('load', () => {
        if (loading) { clearTimeout(loading); loading = null; }
        status.hidden = true;
      });
      frame.addEventListener('error', () => {
        if (loading) { clearTimeout(loading); loading = null; }
        showStatus('Chat could not load. Video playback is still available.');
      });
      panel.append(note, status, frame);
      // A real system-theme change reloads only the existing chat frame (this
      // may discard an unsent cross-origin chat draft). Repeated events for an
      // already-applied theme are no-ops; nothing else is recreated or played.
      if (media && typeof media.addEventListener === 'function') {
        media.addEventListener('change', event => {
          const nextTheme = event.matches === true;
          if (nextTheme === theme) return;
          theme = nextTheme;
          navigate(themedURL());
        });
      }
    }
    // Never move the video or any of its ancestors: that reloads live iframes.
    layout.append(toolbar, panel);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', install, {once: true});
  else install();
}
