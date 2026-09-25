// Browser-only fixture for responsive/emulator UI checks. Native builds use Rust.
const favorites = ['monstercat', 'twitch', 'bobross', 'criticalrole', 'riotgames', 'rocketleague'];
let nextSession = 0;
let state;

function reset() {
  state = {
    mode: 'stopped', display: 'grid', limit: 4, volume: 25, muted: true,
    favorites: favorites.map(login => ({ login, enabled: true, live: true, selected: false })),
    sessions: [], events: ['Browser fixture loaded. Native builds use the Rust controller.'], media: 'simulated',
  };
}
reset();

function reconcile() {
  if (state.mode !== 'running') return;
  const desired = state.favorites.filter(value => value.enabled && value.live).slice(0, state.limit).map(value => value.login);
  state.sessions = desired.map(login => state.sessions.find(value => value.login === login) ?? { login, session: ++nextSession });
  state.favorites.forEach(value => { value.selected = desired.includes(value.login); });
}

function log(message) {
  state.events.unshift(message);
  state.events.length = Math.min(state.events.length, 8);
}

function dispatch(action) {
  const favorite = () => state.favorites.find(value => value.login === action.login);
  switch (action.type) {
    case 'start': state.mode = 'running'; reconcile(); log('Browser fixture started.'); break;
    case 'pause': state.mode = 'paused'; log('Browser fixture paused.'); break;
    case 'stop': state.mode = 'stopped'; state.sessions = []; state.favorites.forEach(value => { value.selected = false; }); log('Browser fixture stopped.'); break;
    case 'set_display': state.display = action.display; break;
    case 'set_limit': state.limit = action.limit; reconcile(); break;
    case 'set_audio': state.volume = action.volume; state.muted = action.muted; break;
    case 'demo_live': favorite().live = action.live; reconcile(); break;
    case 'enable': favorite().enabled = action.enabled; reconcile(); break;
    case 'remove': state.favorites = state.favorites.filter(value => value.login !== action.login); reconcile(); break;
    case 'move': {
      const index = state.favorites.findIndex(value => value.login === action.login);
      const [value] = state.favorites.splice(index, 1); state.favorites.splice(action.position, 0, value); reconcile(); break;
    }
    case 'add': {
      const login = action.input.trim().toLowerCase().replace(/^@/, '');
      if (!/^[a-z0-9_]{1,25}$/.test(login)) throw new Error('Use a Twitch channel login in the browser fixture.');
      state.favorites.push({ login, enabled: true, live: true, selected: false }); reconcile(); break;
    }
    case 'load_demo': reset(); break;
    default: throw new Error(`Unsupported preview action: ${action.type}`);
  }
  return structuredClone(state);
}

export const previewBridge = {
  async invoke(command, payload = {}) {
    if (command === 'get_state') return structuredClone(state);
    if (command === 'dispatch') return dispatch(payload.action);
    throw new Error(`Unsupported preview command: ${command}`);
  },
};
