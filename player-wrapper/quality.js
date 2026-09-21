function createQualityController(initial) {
  'use strict';
  const allowed = new Set(['auto','source','160p','180p','240p','360p','480p','720p','1080p','1440p','2160p']);
  let preference = allowed.has(initial) ? initial : 'auto';
  let player = null, applied = null, suspended = false;

  function variants(values) {
    if (!Array.isArray(values)) return [];
    const seen = new Set();
    return values.slice(0,128).flatMap(value => {
      const id = typeof value === 'string' ? value : value?.group;
      if (typeof id !== 'string' || !/^[a-zA-Z0-9_+.-]{1,64}$/.test(id) || seen.has(id)) return [];
      seen.add(id);
      if (/audio/i.test(id)) return [];
      const name = typeof value?.name === 'string' ? value.name.slice(0,128) : '';
      const match = /^(\d{2,4})p(?:(\d{1,3}))?(?:\b|$)/i.exec(id) || /\b(\d{2,4})p(?:(\d{1,3}))?(?:\b|$)/i.exec(name);
      const height = match ? Number(match[1]) : null;
      return [{id, height, fps: match?.[2] ? Number(match[2]) : 0,
        source: id === 'chunked' || id === 'source', auto: id === 'auto'}];
    });
  }
  function choose(options) {
    if (preference === 'auto') return options.find(v => v.auto) ?? null;
    const video = options.filter(v => !v.auto);
    const source = video.find(v => v.source);
    const sized = video.filter(v => v.height !== null);
    if (preference === 'source') return source ?? sized.sort((a,b) => b.height-a.height || b.fps-a.fps || a.id.localeCompare(b.id))[0] ?? null;
    const target = Number.parseInt(preference,10);
    return sized.sort((a,b) => Math.abs(a.height-target)-Math.abs(b.height-target) ||
      a.height-b.height || a.fps-b.fps || a.id.localeCompare(b.id))[0] ?? source ?? options.find(v => v.auto) ?? null;
  }
  function refresh(paused = suspended) {
    suspended = paused;
    if (!player || suspended) return;
    try {
      // Changing rendition while paused can disturb playback in some players.
      // Retain the preference and apply only after playback resumes normally.
      if (typeof player.isPaused === 'function' && player.isPaused()) return;
      const options = variants(player.getQualities());
      const selected = choose(options);
      if (!selected) { applied = null; return; }
      const key = JSON.stringify([preference, options.map(v => [v.id,v.height,v.fps]).sort()]);
      if (key === applied) return;
      if (player.getQuality() !== selected.id) player.setQuality(selected.id);
      applied = key;
    } catch { /* SDK metadata may arrive later; retry on the next observation. */ }
  }
  return {
    attach(nextPlayer) {
      if (player !== nextPlayer) { player = nextPlayer; applied = null; }
      refresh();
    },
    setPreference(value) {
      if (!allowed.has(value)) return;
      if (value !== preference) { preference = value; applied = null; }
      refresh();
    },
    refresh,
  };
}
