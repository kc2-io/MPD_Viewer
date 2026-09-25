import test from 'node:test';
import assert from 'node:assert/strict';
import { playbackLabel, observedPlaying, secondsAgo } from '../ui/view-model.js';
const player=(extra={})=>({state:'playing',closing:false,report_age_seconds:0,...extra});
test('only fresh actual PLAYING reports count as observed playback',()=>assert.equal(observedPlaying([player(),player({state:'ready'}),player({state:'buffering'})]),1));
test('missing telemetry is unknown, not playing',()=>{assert.equal(observedPlaying([player({report_age_seconds:null})]),0);assert.match(playbackLabel(player({report_age_seconds:null})),/Waiting/);});
test('stale telemetry stops being counted',()=>{assert.equal(observedPlaying([player({report_age_seconds:21})]),0);assert.match(playbackLabel(player({report_age_seconds:21})),/unknown/);});
test('closing windows retain capacity but are not counted as playing',()=>{assert.equal(observedPlaying([player({closing:true})]),0);assert.match(playbackLabel(player({closing:true})),/reserved/);});
test('blocked playback requests an honest user gesture',()=>assert.match(playbackLabel(player({state:'blocked'})),/click/));
test('demo playback is explicitly labeled simulated',()=>assert.match(playbackLabel(player(),true),/^Simulated/));
test('offline player is not confused with authoritative live monitoring',()=>assert.equal(playbackLabel(player({state:'offline'})),'Player reports offline'));
test('paused state is preserved',()=>assert.equal(playbackLabel(player({state:'paused'})),'Paused'));
test('unknown SDK state is not claimed to be playing',()=>assert.equal(playbackLabel(player({state:'mystery'})),'Unknown'));
test('full Twitch page does not fabricate unavailable playback telemetry',()=>assert.equal(
  playbackLabel(player({report_age_seconds:null}),false,false),'Playback managed by Twitch'));
test('closing state remains authoritative without Twitch page telemetry',()=>assert.equal(
  playbackLabel(player({closing:true,report_age_seconds:null}),false,false),'Closing · capacity reserved'));
test('last check handles unknown',()=>assert.equal(secondsAgo(null),'No status check yet'));
test('last check renders seconds',()=>assert.equal(secondsAgo(12),'Last successful check 12s ago'));
test('last check renders minutes',()=>assert.equal(secondsAgo(130),'Last successful check 2m ago'));
