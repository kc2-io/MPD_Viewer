const viewerNumber = new Intl.NumberFormat('en-US');
export function viewerCountLabel(value) {
  if (!value || !Number.isInteger(value.count) || value.count < 0 || value.count > 4294967295) return '';
  return `${viewerNumber.format(value.count)} ${value.count === 1 ? 'viewer' : 'viewers'}${value.stale ? ' (stale)' : ''}`;
}

// Presentation helpers only. Selection policy lives exclusively in mpd-core.
export function playbackLabel(player, demo = false, telemetry = true) {
  if (player.closing) return 'Closing · capacity reserved';
  if (!telemetry) return 'Playback managed by Twitch';
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

export function rescanLabel(minutes) {
  return `Every ${minutes} min`;
}

export function timerLabel(timer) {
  if (!timer || timer.state === 'unlimited') return 'Always · no assignment timer';
  if (timer.state === 'closing_for_rotation') return 'Timer reached · rotating to next live channel';
  if (timer.state === 'waiting_for_alternative') return 'Time reached · waiting for another live channel';
  const seconds = Number.isSafeInteger(timer.remaining_seconds) && timer.remaining_seconds >= 0 ? timer.remaining_seconds : 0;
  const clock = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2,'0')}`;
  return `${timer.state === 'automation_paused' ? 'Automation paused · ' : ''}${clock} assigned time left`;
}
