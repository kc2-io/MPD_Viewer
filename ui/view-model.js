// Presentation helpers only. Selection policy lives exclusively in mpd-core.
export function playbackLabel(player, demo = false) {
  if (player.closing) return 'Closing · capacity reserved';
  if (player.report_age_seconds == null) return 'Waiting for player telemetry';
  if (player.report_age_seconds > 20) return 'Telemetry stale · playback unknown';
  const labels = { loading:'Loading', ready:'Ready', buffering:'Buffering', playing:'Playing',
    paused:'Paused', blocked:'Needs a click in the player', offline:'Player reports offline',
    ended:'Playback ended', error:'Player error · inspect or retry' };
  return `${demo ? 'Simulated · ' : ''}${labels[player.state] ?? 'Unknown'}`;
}
export function observedPlaying(players) {
  return players.filter(p => !p.closing && p.state === 'playing' &&
    p.report_age_seconds != null && p.report_age_seconds <= 20).length;
}
export function secondsAgo(value) {
  if (value == null) return 'No status check yet';
  if (value < 60) return `Last successful check ${value}s ago`;
  return `Last successful check ${Math.floor(value / 60)}m ago`;
}
