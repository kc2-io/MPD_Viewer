// Browser UI checks with a mocked native bridge; not website login evidence.
const {chromium} = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = process.env.MPD_TEST_ROOT || path.resolve(__dirname, '..');
(async () => {
  const browser = await chromium.launch({channel: process.env.MPD_TEST_BROWSER || 'msedge', headless:true});
  try {
    const page = await browser.newPage({viewport:{width:1280,height:1000}});
    const errors=[]; page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => {
      const url=new URL(route.request().url());
      if(url.origin!=='https://mpd.test')return route.abort();
      const name=url.pathname==='/'?'index.html':url.pathname.slice(1);
      const file=path.join(root,'ui',name);
      return fs.existsSync(file)?route.fulfill({path:file}):route.fulfill({status:404,body:''});
    });
    await page.addInitScript({path:path.join(root,'tests/ui-fixture.js')});
    await page.goto('https://mpd.test/');
    await page.waitForSelector('.player-card');
    assert(await page.locator('#connect').isDisabled());
    assert.equal(await page.locator('#client-id').count(),0);
    assert.equal(await page.locator('#viewer-signin').count(),0);
    await page.evaluate(() => {
      window.__fixture.settings.demo=false;
      window.__fixture.viewer={backend:'twitch-page',telemetry:false,media_controls:false,twitch_channel_page:true};
      window.__fixture.player_origin='https://www.twitch.tv/';
    });
    await page.waitForFunction(() => !document.getElementById('connect').disabled);
    assert(await page.locator('#media-controls').isHidden());
    assert(await page.locator('#twitch-page-note').isVisible());
    assert.match(await page.locator('#twitch-page-note').textContent(),/Dark Theme/);
    assert.match(await page.locator('#twitch-page-note').textContent(),/Central volume and quality controls are unavailable/);
    await page.evaluate(() => {window.__fixture.viewer={backend:'twitch-embed',telemetry:true,media_controls:true,twitch_channel_page:false};});
    await page.waitForFunction(() => !document.getElementById('media-controls').hidden);
    assert(await page.locator('#twitch-page-note').isHidden());
    await page.evaluate(() => {window.__fixture.viewer={backend:'twitch-page',telemetry:false,media_controls:false,twitch_channel_page:true};});
    await page.waitForFunction(() => document.getElementById('media-controls').hidden);
    assert.equal(await page.locator('#viewer-backend').textContent(),'twitch-page');
    assert.equal(await page.locator('#playing-heading').textContent(),'TWITCH CHANNEL PAGE');
    assert.match(await page.locator('.player-state').first().textContent(),/playback managed by Twitch/);
    await page.locator('#connect').click();
    await page.waitForFunction(() => window.__actions.length===1);
    assert.deepEqual(await page.evaluate(() => window.__actions),[{type:'connect'}]);
    await page.evaluate(() => {window.__fixture.auth_pending=true;});
    await page.waitForFunction(() => document.getElementById('connect').textContent==='Connecting Twitch…');
    assert(await page.locator('#connect').isDisabled());
    assert.equal(await page.locator('#disconnect').textContent(),'Cancel');
    await page.evaluate(() => {window.__fixture.user_code='TESTCODE';});
    await page.waitForFunction(() => document.getElementById('connect').textContent==='Continue in Twitch');
    await page.locator('#connect').click();
    assert.deepEqual(await page.evaluate(() => window.__actions.at(-1)),{type:'connect'});
    await page.locator('#disconnect').click();
    assert.deepEqual(await page.evaluate(() => window.__actions.at(-1)),{type:'disconnect'});
    await page.evaluate(() => {window.__fixture.auth_pending=false;window.__fixture.user_code=null;window.__fixture.connected_as='monitor_test';});
    await page.waitForFunction(() => document.getElementById('auth-status').textContent==='Monitoring as monitor_test');
    assert.equal(await page.locator('#connect').textContent(),'Reconnect Twitch');
    assert.equal(await page.locator('.player-card').count(),3);
    await page.evaluate(() => {window.__fixture.connected_as=null;window.__fixture.error='Could not remove saved authorization. Retry Disconnect.';});
    await page.waitForFunction(() => document.getElementById('auth-status').textContent==='Not connected');
    assert(await page.locator('#disconnect').isEnabled());
    await page.locator('#disconnect').click();
    assert.deepEqual(await page.evaluate(() => window.__actions.at(-1)),{type:'disconnect'});
    assert.match(await page.locator('#auth-panel').textContent(),/Disconnect removes saved monitoring authorization/);
    assert.deepEqual(errors,[]);
    console.log('PASS unified Connect, pending/reopen/cancel states, no client ID override, monitoring-only status, player cards retained, no JS errors');
  } finally {await browser.close();}
})().catch(error => {console.error(error);process.exitCode=1;});
