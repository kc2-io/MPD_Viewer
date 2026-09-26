import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readdir, realpath } from 'node:fs/promises';
import path from 'node:path';
import { sanitize } from './support.mjs';

const WIDTH = 1280;
const HEIGHT = 800;
const FRAME_RATE = 8;
const SEGMENT_SECONDS = 60;
const SEGMENT_LIMIT = 32 * 1024 * 1024;
const SCALE = `scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease:force_divisible_by=2,format=yuv420p`;
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const exited = child => child.exitCode !== null || child.signalCode !== null;
const ENV_KEYS = ['PATH', 'Path', 'PATHEXT', 'SystemRoot', 'SYSTEMROOT', 'WINDIR', 'TEMP', 'TMP', 'TMPDIR',
  'HOME', 'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'LANG', 'LC_ALL', 'DISPLAY', 'XAUTHORITY',
  'DYLD_LIBRARY_PATH', 'LD_LIBRARY_PATH'];
const recorderEnvironment = () => Object.fromEntries(ENV_KEYS.filter(key => process.env[key] !== undefined).map(key => [key, process.env[key]]));

function executablePath() {
  const override = process.env.MPD_E2E_FFMPEG;
  if (override && !path.isAbsolute(override)) throw new Error('MPD_E2E_FFMPEG must be an absolute path');
  const locator = process.platform === 'win32' ? 'where.exe' : 'which';
  if (override) return override;
  const found = spawnSync(locator, ['ffmpeg'], { encoding: 'utf8', windowsHide: true, timeout: 5000 });
  if (found.status !== 0 || !found.stdout.trim()) throw new Error('FFmpeg executable unavailable');
  return found.stdout.trim().split(/\r?\n/)[0];
}

function screenInput(executable) {
  if (process.platform === 'win32') return { backend: 'gdigrab', args: ['-f', 'gdigrab', '-framerate', String(FRAME_RATE), '-i', 'desktop'] };
  if (process.platform === 'linux') {
    if (!process.env.DISPLAY) throw new Error('DISPLAY unavailable for x11grab');
    const display = process.env.DISPLAY.includes('.') ? process.env.DISPLAY : `${process.env.DISPLAY}.0`;
    return { backend: 'x11grab', args: ['-f', 'x11grab', '-video_size', '1600x1000', '-framerate', String(FRAME_RATE), '-i', display] };
  }
  if (process.platform === 'darwin') {
    const list = spawnSync(executable, ['-hide_banner', '-f', 'avfoundation', '-list_devices', 'true', '-i', ''], { encoding: 'utf8', windowsHide: true, timeout: 10000 });
    const lines = `${list.stdout || ''}\n${list.stderr || ''}`.split(/\r?\n/);
    const screens = lines.filter(line => /\[\d+\].*Capture screen/i.test(line));
    const index = screens[0]?.match(/\[(\d+)\]/)?.[1];
    if (screens.length !== 1 || index === undefined) throw new Error('Expected exactly one AVFoundation screen device');
    return { backend: 'avfoundation', args: ['-f', 'avfoundation', '-framerate', String(FRAME_RATE), '-capture_cursor', '1', '-i', `${index}:none`] };
  }
  throw new Error('No desktop recorder backend for this platform');
}

async function hashExecutable(executable) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(executable)) hash.update(chunk);
  return hash.digest('hex');
}

export class VideoRecorder {
  constructor(output) {
    this.output = output;
    this.child = null;
    this.closed = null;
    this.started = 0;
    this.info = { requested: process.env.MPD_E2E_RECORD_VIDEO === '1', backend: null, executable: null,
      version: null, executableSha256: null, container: 'mp4', codec: 'h264', width: WIDTH, height: HEIGHT,
      frameRate: FRAME_RATE, audio: false, segments: [], stopMode: 'not-requested' };
  }

  async start() {
    if (!this.info.requested) return;
    try {
      const executable = await realpath(executablePath());
      const version = spawnSync(executable, ['-version'], { encoding: 'utf8', windowsHide: true, timeout: 5000 });
      if (version.status !== 0) throw new Error('FFmpeg version probe failed');
      const source = screenInput(executable);
      this.info.backend = source.backend;
      this.info.executable = executable;
      this.info.version = version.stdout.split(/\r?\n/)[0].slice(0, 160);
      this.info.executableSha256 = await hashExecutable(executable);
      const args = ['-hide_banner', '-nostats', '-loglevel', 'error', '-y', ...source.args,
        '-an', '-vf', SCALE, '-c:v', 'libx264', '-preset', 'ultrafast', '-tune', 'zerolatency',
        '-b:v', '800k', '-maxrate', '900k', '-bufsize', '1800k', '-g', '8', '-keyint_min', '8', '-sc_threshold', '0',
        '-f', 'segment', '-segment_time', String(SEGMENT_SECONDS), '-reset_timestamps', '1',
        '-segment_format', 'mp4', '-segment_format_options', 'movflags=+frag_keyframe+empty_moov:flush_packets=1',
        path.join(this.output, 'video-%03d.mp4')];
      const child = spawn(executable, args, { shell: false, detached: false, windowsHide: true,
        stdio: ['pipe', 'ignore', 'pipe'], env: recorderEnvironment() });
      this.child = child;
      this.started = Date.now();
      let diagnostic = '';
      child.stdin.on('error', () => {});
      child.stderr.on('data', chunk => { diagnostic = (diagnostic + chunk.toString()).slice(-2048); });
      this.closed = new Promise(resolve => {
        child.once('error', error => { this.info.error = `Recorder launch failed: ${error.code || 'unknown'}`; });
        child.once('close', (code, signal) => {
          if (!this.stopping && !this.info.error) this.info.error = `Recorder exited before shutdown (${code ?? signal ?? 'unknown'})`;
          if (code !== 0 && !this.info.error) this.info.error = `Recorder exit ${code ?? signal ?? 'unknown'}`;
          if (diagnostic && this.info.error) this.info.error += `; ${sanitize(diagnostic).replace(/[\r\n]+/g, ' ').replace(/https?:\/\/\S+/g, '[url]').slice(0, 256)}`;
          resolve();
        });
      });
      await Promise.race([this.closed, wait(1000)]);
      this.info.stopMode = exited(child) ? 'early-exit' : 'recording';
    } catch (error) {
      this.info.error = String(error.message).slice(0, 256);
      this.info.stopMode = 'start-failed';
    }
  }

  async stop(interrupted = false) {
    const child = this.child;
    if (child && !exited(child)) {
      this.stopping = true;
      try { child.stdin.write('q\n'); child.stdin.end(); } catch {}
      await Promise.race([this.closed, wait(interrupted ? 2000 : 3000)]);
      if (exited(child)) this.info.stopMode = 'graceful';
      else {
        child.kill('SIGTERM');
        await Promise.race([this.closed, wait(2000)]);
        if (exited(child)) this.info.stopMode = 'escalated';
        else {
          child.kill('SIGKILL');
          await Promise.race([this.closed, wait(2000)]);
          this.info.stopMode = exited(child) ? 'escalated' : 'lost';
          if (!exited(child)) this.info.error = 'Recorder did not exit after owned-child escalation';
        }
      }
    } else if (child && this.info.stopMode === 'recording') {
      this.info.stopMode = interrupted ? 'escalated' : 'early-exit';
      if (interrupted && this.info.error?.startsWith('Recorder exited before shutdown')) delete this.info.error;
    }
    await this.collectSegments();
    return this.info;
  }

  async collectSegments() {
    const names = (await readdir(this.output)).filter(name => /^video-\d{3}\.mp4$/.test(name)).sort();
    const elapsed = this.started ? (Date.now() - this.started) / 1000 : 0;
    this.info.segments = [];
    for (const name of names.slice(0, 32)) {
      const file = path.join(this.output, name);
      const metadata = await lstat(file);
      const index = this.info.segments.length;
      this.info.segments.push({ name, bytes: metadata.size, durationSeconds: null,
        estimatedDurationSeconds: Math.min(SEGMENT_SECONDS, Math.max(0, elapsed - index * SEGMENT_SECONDS)), validated: null });
      if (!metadata.isFile() || metadata.size === 0 || metadata.size > SEGMENT_LIMIT) this.info.error = 'Recorder segment missing, empty or over size limit';
    }
    if (this.info.requested && !names.length && !this.info.error) this.info.error = 'Recorder produced no segments';
    if (names.length > 32) this.info.error = 'Recorder produced more than 32 segments';
  }
}
