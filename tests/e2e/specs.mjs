import assert from 'node:assert/strict';
import { click, input, byLabel, order, until, visibleText, windows, delay, selectValue, dragBefore, documentTitle } from './support.mjs';

const expectedOrder = ['bravo_demo', 'alpha_demo', 'charlie_demo', 'delta_demo'];
export async function demoSmoke(app, test, extended) {
  let browser = await app.start();
  await test('fresh isolated manager remains stopped', async () => {
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.deepEqual(await order(browser), []);
    await windows(browser, 1);
  });
  await test('load demo, add/remove and drag ranking through real UI', async () => {
    await selectValue(browser, '#source', 'demo');
    await click(browser, '#load-demo');
    await until(async () => (await order(browser)).length === 4, 'Demo channels not loaded');
    await input(browser, '#channel', 'e2e_temporary'); await click(browser, '#add-form button');
    await until(async () => (await order(browser)).includes('e2e_temporary'), 'Added channel missing');
    await click(browser, byLabel('Remove e2e_temporary'));
    await until(async () => (await order(browser)).length === 4, 'Removed channel retained');
    await dragBefore(browser, 'bravo_demo', 'alpha_demo');
    await until(async () => JSON.stringify(await order(browser)) === JSON.stringify(expectedOrder), 'Dragged rank not persisted into rendered state');
    await click(browser, byLabel('Enable alpha_demo'));
    await until(async () => (await browser.$('li[data-login="alpha_demo"]')).getAttribute('aria-disabled').then(value => value === 'true'), 'Enable change missing');
  });
  await test('invalid channel is rejected without changing saved favorites', async () => {
    const before = await order(browser);
    await input(browser, '#channel', 'not a valid channel!');
    await click(browser, '#add-form button');
    await until(async () => (await browser.$('#error')).isDisplayed(), 'Invalid channel did not report an error');
    assert.deepEqual(await order(browser), before);
    await click(browser, '#dismiss-error');
    await input(browser, '#channel', '');
  });
  await test('synthetic HTML5 drag persists order through real UI handler', async () => {
    await dragBefore(browser, 'charlie_demo', 'bravo_demo');
    await until(async () => (await order(browser))[0] === 'charlie_demo', 'Drag rank not saved');
    await dragBefore(browser, 'bravo_demo', 'charlie_demo');
    await until(async () => (await order(browser))[0] === 'bravo_demo', 'First restored drag rank not saved');
    await dragBefore(browser, 'alpha_demo', 'charlie_demo');
    await until(async () => JSON.stringify(await order(browser)) === JSON.stringify(expectedOrder), 'Restored drag rank not saved');
  });
  await test('timer presets and settings save through production dispatch', async () => {
    await click(browser, byLabel('Set bravo_demo timer to 10 minutes'));
    await until(async () => (await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('10 min assigned time'), 'Timer preset not saved');
    await input(browser, '#limit', 2); await click(browser, 'h1');
    await until(async () => (await visibleText(browser, '#session-count')).includes('/ 2'), 'Limit not saved');
    await input(browser, '#rescan', 5); await click(browser, 'h1');
    await until(() => browser.execute(async () => (await window.__TAURI__.core.invoke('get_state')).settings.rescan_minutes === 5), 'Rescan interval not saved');
    await selectValue(browser, '#quality', '360p');
  });
  await test('invalid timer is rejected and Always removes the saved timer', async () => {
    const selector = byLabel('Assignment timer minutes for bravo_demo');
    await input(browser, selector, '0');
    await click(browser, byLabel('Save timer for bravo_demo'));
    assert.equal(await browser.execute(selector => document.querySelector(selector).validity.valid, selector), false);
    assert.ok((await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('10 min assigned time'));
    await click(browser, byLabel('Remove timer for bravo_demo'));
    await until(async () => !(await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('10 min assigned time'), 'Always did not remove the timer');
    assert.equal(await (await browser.$(selector)).getValue(), '');
    await click(browser, byLabel('Set bravo_demo timer to 10 minutes'));
    await until(async () => (await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('10 min assigned time'), 'Timer preset was not restored');
  });
  await test('two viewer handles render; pause/resume and Stop clean up', async () => {
    await click(browser, '#start');
    await until(async () => await visibleText(browser, '#run-status') === 'Running', 'Start did not enter running state');
    const handles = await windows(browser, 3);
    const channels = [];
    for (const handle of handles.filter(handle => handle !== app.manager)) {
      await browser.switchToWindow(handle);
      const channel = await app.viewerReady({ title: 'MPD Player', channels: ['bravo_demo', 'charlie_demo'], demo: true });
      channels.push(channel);
    }
    assert.deepEqual(channels.sort(), ['bravo_demo', 'charlie_demo']);
    await browser.switchToWindow(app.manager);
    await click(browser, '#pause');
    await until(async () => await visibleText(browser, '#run-status') === 'Automation paused', 'Pause did not render');
    await windows(browser, 3);
    await click(browser, '#start');
    await until(async () => await visibleText(browser, '#run-status') === 'Running', 'Resume did not render');
    assert.equal(await app.screenshot('demo-two-windows'), 3, 'Screenshot capture must work for every native window');
    await click(browser, '#stop');
    await windows(browser, 1);
    await until(async () => (await browser.$$('#players > article')).length === 0, 'Stopped session cards retained');
  });
  if (extended) await test('real timer preserves paused time and rotates without observed capacity overshoot', async () => {
    await input(browser, '#limit', 1); await click(browser, 'h1');
    await input(browser, byLabel('Assignment timer minutes for bravo_demo'), 1);
    await click(browser, byLabel('Save timer for bravo_demo'));
    await until(async () => (await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('1 min assigned time'), 'One-minute timer not saved');
    const start = Date.now();
    await click(browser, '#start'); await windows(browser, 2);
    await until(async () => await visibleText(browser, '#players > article > strong') === 'bravo_demo', 'Expected first timed channel');
    const timerText = () => browser.execute(() => document.querySelector('.player-timer')?.textContent?.trim() ?? null);
    await until(async () => /0:5[0-9] assigned time left/.test(await timerText()), 'Expected a running assignment countdown');
    await click(browser, '#pause');
    await until(async () => (await timerText()).startsWith('Automation paused'), 'Timer did not show paused accounting');
    const pausedLabel = await timerText();
    const pauseStart = Date.now();
    while (Date.now() - pauseStart < 5000) {
      assert.equal(await timerText(), pausedLabel, 'Assignment time decreased while automation was paused');
      assert.equal((await browser.getWindowHandles()).length, 2, 'Paused assignment changed native capacity');
      await delay(150);
    }
    const pausedDuration = Date.now() - pauseStart;
    await click(browser, '#start');
    const remaining = text => { const match = text.match(/(\d+):(\d{2}) assigned time left/); return match ? Number(match[1]) * 60 + Number(match[2]) : null; };
    await until(async () => { const text = await timerText(); const seconds = remaining(text); return !text.startsWith('Automation paused') && seconds !== null && seconds < remaining(pausedLabel); }, 'Assignment countdown did not resume');
    let observedOvershoot = false;
    await until(async () => {
      if ((await browser.getWindowHandles()).length > 2) observedOvershoot = true;
      return await visibleText(browser, '#players > article > strong') === 'charlie_demo';
    }, 'Real elapsed timer did not rotate', 85000);
    assert.equal(observedOvershoot, false, 'Observed native windows exceeded capacity during timer replacement');
    assert.ok(Date.now() - start >= 59000 + pausedDuration, 'Rotation did not preserve the paused assignment duration');
    await windows(browser, 2); await click(browser, '#stop'); await windows(browser, 1);
    await click(browser, byLabel('Set bravo_demo timer to 10 minutes'));
    await until(() => browser.execute(() => document.querySelector('li[data-login="bravo_demo"]')?.textContent?.includes('10 min assigned time')), 'Restored timer was not acknowledged by the Rust view');
    await input(browser, '#limit', 2); await click(browser, 'h1');
    await until(() => browser.execute(() => document.querySelector('#session-count')?.textContent?.includes('/ 2')), 'Restored capacity was not acknowledged by the Rust view');
  });
  await app.stop();
  browser = await app.start();
  await test('full process restart restores preferences without opening viewers', async () => {
    assert.deepEqual(await order(browser), expectedOrder);
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.equal(await (await browser.$('#limit')).getValue(), '2');
    assert.equal(await (await browser.$('#rescan')).getValue(), '5');
    assert.equal(await (await browser.$('#quality')).getValue(), '360p');
    assert.equal(await (await browser.$(byLabel('Assignment timer minutes for bravo_demo'))).getValue(), '10');
    assert.equal(await (await browser.$('li[data-login="alpha_demo"]')).getAttribute('aria-disabled'), 'true');
    await windows(browser, 1); assert.equal(await app.screenshot('restart-persistence'), 1);
  });
}

export async function webSmoke(app, test, extended) {
  let browser = await app.start('web');
  await test('full-page mode hides central media controls and starts two local viewers', async () => {
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.equal(await (await browser.$('#media-controls')).isDisplayed(), false);
    assert.equal(await (await browser.$('#twitch-page-note')).isDisplayed(), true);
    const windowMute = await browser.execute(async () => (await window.__TAURI__.core.invoke('get_state')).viewer.window_mute_controls);
    assert.equal(await (await browser.$('#window-mute-controls')).isDisplayed(), windowMute);
    if (windowMute) {
      await click(browser, '#window-muted');
      await until(() => browser.execute(async () => (await window.__TAURI__.core.invoke('get_state')).settings.muted), 'Global page mute was not persisted before launch');
    } else {
      assert.match(await visibleText(browser, '#twitch-page-note'), /unavailable on this platform/);
    }
    await click(browser, '#start');
    const handles = await windows(browser, 3);
    const channels = [];
    for (const handle of handles.filter(handle => handle !== app.manager)) {
      await browser.switchToWindow(handle);
      channels.push(await fixtureViewerReady(app));
      await browser.execute(nonce => localStorage.setItem('mpd-e2e-test-nonce', nonce), app.runId);
    }
    assert.deepEqual(channels.sort(), ['alpha_fixture', 'bravo_fixture']);
    await browser.switchToWindow(app.manager);
    if (windowMute) {
      await until(() => browser.execute(async () => (await window.__TAURI__.core.invoke('get_state')).players.every(player => player.window_muted === true)), 'New full-page windows did not inherit confirmed global mute');
      const handlesBeforeMuteChanges = (await browser.getWindowHandles()).sort();
      assert.equal(await (await browser.$(byLabel('alpha_fixture page window is muted; global mute control is on'))).isEnabled(), false);
      await click(browser, '#window-muted');
      await until(() => browser.execute(async () => { const state=await window.__TAURI__.core.invoke('get_state');return !state.settings.muted&&state.players.every(player=>player.window_muted===false); }), 'Global unmute did not restore individual page state');
      await click(browser, byLabel('Mute alpha_fixture page window'));
      await until(() => browser.execute(async () => { const player=(await window.__TAURI__.core.invoke('get_state')).players.find(player=>player.login==='alpha_fixture');return player?.window_muted===true&&player?.window_mute_override===true; }), 'Per-page native mute was not confirmed');
      assert.deepEqual((await browser.getWindowHandles()).sort(), handlesBeforeMuteChanges, 'Mute controls replaced a native viewer window');
    }
    assert.equal(await app.screenshot('web-two-windows'), 3);
  });
  await test('fixture viewer count updates through Check now without replacing viewers', async () => {
    await until(async () => (await visibleText(browser, 'li[data-login="alpha_fixture"]')).includes('100 viewers'), 'Initial live viewer count missing');
    const handles = await browser.getWindowHandles();
    await app.fixtureState({ count: 777 });
    await click(browser, '#refresh');
    await until(async () => (await visibleText(browser, 'li[data-login="alpha_fixture"]')).includes('777 viewers'), 'Updated live viewer count missing');
    assert.deepEqual((await browser.getWindowHandles()).sort(), [...handles].sort());
  });
  if (extended) await test('API outage marks counts stale, preserves viewers and recovers after backoff', async () => {
    const handles = await browser.getWindowHandles();
    await app.fixtureState({ fail_api: true, count: 777 });
    await click(browser, '#refresh');
    await until(async () => (await visibleText(browser, 'li[data-login="alpha_fixture"]')).includes('(stale)'), 'Failed API did not mark count stale');
    assert.equal(await (await browser.$('#error')).isDisplayed(), true);
    assert.deepEqual((await browser.getWindowHandles()).sort(), [...handles].sort());
    await app.fixtureState({ count: 888 });
    await click(browser, '#refresh');
    await until(async () => (await visibleText(browser, 'li[data-login="alpha_fixture"]')).includes('888 viewers'), 'API did not recover through production backoff', 45000);
    assert.deepEqual((await browser.getWindowHandles()).sort(), [...handles].sort());
    await click(browser, '#dismiss-error');
  });
  await test('instrumented full-page caller is denied real manager IPC', async () => {
    const handle = (await browser.getWindowHandles()).find(value => value !== app.manager);
    await browser.switchToWindow(handle);
    const probes = await browser.execute(async () => {
      const invoke = window.__TAURI__?.core?.invoke;
      if (!invoke) return { missingNativeBridge: true };
      const results = [];
      for (const [command, args] of [['get_state', {}], ['dispatch', { action: { type: 'stop' } }], ['player_report', { report: {} }]]) {
        try { await invoke(command, args); results.push({ command, denied: false }); }
        catch (error) { results.push({ command, denied: /not allowed|denied|forbidden|permission|not authorized/i.test(String(error)) }); }
      }
      return { results };
    });
    assert.equal(probes.missingNativeBridge, undefined, 'Native IPC bridge must exist for a genuine denial probe');
    assert.deepEqual(probes.results, ['get_state', 'dispatch', 'player_report'].map(command => ({ command, denied: true })));
    await browser.switchToWindow(app.manager);
    assert.equal(await visibleText(browser, '#run-status'), 'Running');
  });
  await test('native theme API changes manager scheme without replacing viewers', async () => {
    const handles = await browser.getWindowHandles();
    await app.nativeCommand({ action: 'theme', label: app.manager, theme: 'dark' });
    await until(() => browser.execute(() => matchMedia('(prefers-color-scheme: dark)').matches), 'Native dark scheme not applied');
    assert.deepEqual((await browser.getWindowHandles()).sort(), [...handles].sort());
    await app.nativeCommand({ action: 'theme', label: app.manager, theme: 'light' });
    await until(async () => !(await browser.execute(() => matchMedia('(prefers-color-scheme: dark)').matches)), 'Native light scheme not applied');
    assert.deepEqual((await browser.getWindowHandles()).sort(), [...handles].sort());
  });
  await test('native close API skips a full-page assignment and replaces its window', async () => {
    const old = (await browser.getWindowHandles()).find(handle => handle !== app.manager);
    await app.nativeCommand({ action: 'close', label: old });
    await until(async () => { const current = await browser.getWindowHandles(); return current.length === 3 && !current.includes(old); }, 'Closed viewer was not replaced');
    await until(async () => (await visibleText(browser, '#favorites')).includes('SKIPPED'), 'Native CloseRequested did not produce skip');
  });
  await test('full-page native windows honor capacity reduction and Stop', async () => {
    await input(browser, '#limit', 1); await click(browser, 'h1');
    await windows(browser, 2);
    await until(async () => (await browser.$$('#players > article')).length === 1, 'Capacity reduction not reflected');
    await click(browser, '#stop'); await windows(browser, 1);
  });
  await app.stop();
  browser = await app.start('web');
  await test('full-page preferences restart stopped with no viewer handles', async () => {
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.equal(await (await browser.$('#limit')).getValue(), '1');
    assert.equal(await (await browser.$('#media-controls')).isDisplayed(), false);
    await windows(browser, 1);
    await click(browser, '#start');
    const handles = await windows(browser, 2);
    await browser.switchToWindow(handles.find(handle => handle !== app.manager));
    await fixtureViewerReady(app);
    assert.equal(await browser.execute(() => localStorage.getItem('mpd-e2e-test-nonce')), app.runId, 'Disposable browser profile did not survive full process restart');
    await browser.switchToWindow(app.manager);
    await click(browser, '#stop'); await windows(browser, 1);
  });
  await test('enable changes replace and preempt the active priority under capacity one', async () => {
    await click(browser, '#start'); await windows(browser, 2);
    await until(async () => await visibleText(browser, '#players > article > strong') === 'alpha_fixture', 'Highest priority fixture not selected');
    await click(browser, byLabel('Enable alpha_fixture'));
    await until(async () => await visibleText(browser, '#players > article > strong') === 'bravo_fixture', 'Disabling active favorite did not select next priority');
    await windows(browser, 2);
    await click(browser, byLabel('Enable alpha_fixture'));
    await until(async () => await visibleText(browser, '#players > article > strong') === 'alpha_fixture', 'Reenabled higher priority did not preempt');
    await windows(browser, 2); await click(browser, '#stop'); await windows(browser, 1);
  });
  await test('native open failure offers Retry and recovers the preferred viewer', async () => {
    await app.fixtureState({ fail_open: ['alpha_fixture'] });
    await click(browser, '#start');
    await until(async () => await (await browser.$('li[data-login="alpha_fixture"] [aria-label="Retry alpha_fixture"]')).isExisting(), 'Failed native open did not offer Retry');
    await until(async () => await visibleText(browser, '#players > article > strong') === 'bravo_fixture', 'Failed open did not select the next live priority');
    const before = await windows(browser, 2);
    await browser.switchToWindow(before.find(handle => handle !== app.manager));
    assert.equal(await fixtureViewerReady(app), 'bravo_fixture');
    await browser.switchToWindow(app.manager);
    await app.fixtureState({});
    await click(browser, 'li[data-login="alpha_fixture"] [aria-label="Retry alpha_fixture"]');
    await until(async () => await visibleText(browser, '#players > article > strong') === 'alpha_fixture', 'Retry did not reopen the preferred live channel');
    const after = await windows(browser, 2);
    assert.notDeepEqual([...after].sort(), [...before].sort(), 'Retry did not replace the actual native viewer');
    await browser.switchToWindow(after.find(handle => handle !== app.manager));
    assert.equal(await fixtureViewerReady(app), 'alpha_fixture');
    await browser.switchToWindow(app.manager);
    assert.equal(await (await browser.$('li[data-login="alpha_fixture"] [aria-label="Retry alpha_fixture"]')).isExisting(), false, 'Successful Retry retained the favorite open error');
    await click(browser, '#stop'); await windows(browser, 1);
  });
  await test('native manager close exits with an active viewer', async () => {
    await click(browser, '#start');
    const handles = await windows(browser, 2);
    await browser.switchToWindow(handles.find(handle => handle !== app.manager));
    await fixtureViewerReady(app);
    await browser.switchToWindow(app.manager);
    await app.nativeCommand({ action: 'close', label: app.manager });
    await until(() => app.child.exitCode !== null || app.child.signalCode !== null, 'Manager close did not exit the process');
    await assert.rejects(() => browser.getWindowHandles(), 'Driver handles must become unavailable after native manager exit');
  });
}


export async function authSmoke(app, test) {
  let browser = await app.start('auth');
  if (process.platform !== 'win32') {
    await test('non-Windows Connect reports the existing unsupported capability', async () => {
      await click(browser, '#connect');
      await until(async () => /Windows|not supported|unavailable/i.test(await visibleText(browser, '#error-text')), 'Unsupported Connect did not report its limitation');
      await until(async () => await visibleText(browser, '#auth-status') === 'Not connected', 'Authorization did not settle disconnected', 30000);
      await windows(browser, 1);
    });
    return;
  }
  await test('Windows Connect authorizes only the local fake OAuth fixture', async () => {
    await until(async () => await visibleText(browser, '#auth-status') === 'Not connected', 'Authorization did not settle disconnected', 30000);
    await click(browser, '#connect');
    const handles = await windows(browser, 2);
    await browser.switchToWindow(handles.find(handle => handle !== app.manager));
    await documentTitle(browser, 'MPD E2E authorization fixture');
    await until(async () => await visibleText(browser, '#authorize') === 'Authorize simulation', 'Local fake activation UI missing');
    await click(browser, '#authorize');
    await browser.switchToWindow(app.manager);
    await until(async () => (await visibleText(browser, '#auth-status')).startsWith('Monitoring as '), 'Fake authorization did not complete', 30000);
    await windows(browser, 1);
  });
  await app.stop(); browser = await app.start('auth');
  await test('Windows native vault restores fake authorization after process restart', async () => {
    await until(async () => (await visibleText(browser, '#auth-status')).startsWith('Monitoring as '), 'Fake vault authorization was not restored', 30000);
    await windows(browser, 1);
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    await click(browser, '#disconnect');
    await until(async () => await visibleText(browser, '#auth-status') === 'Not connected', 'Disconnect did not clear authorization');
  });
  await app.stop(); browser = await app.start('auth');
  await test('Windows Disconnect deletes fake vault authorization across restart', async () => {
    await until(async () => await visibleText(browser, '#auth-status') === 'Not connected', 'Authorization did not settle disconnected', 30000);
    assert.equal(await (await browser.$('#error')).isDisplayed(), false, 'Credential recovery error must not masquerade as successful Disconnect');
    await windows(browser, 1);
  });
}

async function fixtureViewerReady(app) {
  return app.viewerReady({ title: 'MPD E2E local viewer', channels: ['alpha_fixture', 'bravo_fixture', 'charlie_fixture'] });
}

export async function embeddedSmoke(app, test) {
  const browser = await app.start('web', ['--embedded-viewer']);
  await test('explicit embedded launch exposes media controls and wrapper windows', async () => {
    assert.equal(await (await browser.$('#media-controls')).isDisplayed(), true);
    assert.equal(await (await browser.$('#twitch-page-note')).isDisplayed(), false);
    await selectValue(browser, '#quality', '360p');
    await click(browser, '#start');
    const handles = await windows(browser, 3);
    const channels = [];
    for (const handle of handles.filter(handle => handle !== app.manager)) {
      assert.match(handle, /^player-/);
      await browser.switchToWindow(handle);
      channels.push(await app.viewerReady({ title: 'MPD Player', channels: ['alpha_fixture', 'bravo_fixture'], demo: true }));
    }
    assert.deepEqual(channels.sort(), ['alpha_fixture', 'bravo_fixture']);
    await browser.switchToWindow(app.manager);
    assert.equal(await app.screenshot('explicit-embedded'), 3);
    await click(browser, '#stop'); await windows(browser, 1);
  });
}
