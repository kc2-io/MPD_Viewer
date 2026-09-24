import assert from 'node:assert/strict';
import { click, input, byLabel, order, until, visibleText, windows, delay, selectValue, dragBefore } from './support.mjs';

const expectedOrder = ['bravo_demo', 'alpha_demo', 'charlie_demo', 'delta_demo'];
export async function demoSmoke(app, test, extended) {
  let browser = await app.start();
  await test('fresh isolated manager remains stopped', async () => {
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.deepEqual(await order(browser), []);
    await windows(browser, 1);
  });
  await test('load demo, add/remove and arrow ranking through real UI', async () => {
    await selectValue(browser, '#source', 'demo');
    await click(browser, '#load-demo');
    await until(async () => (await order(browser)).length === 4, 'Demo channels not loaded');
    await input(browser, '#channel', 'e2e_temporary'); await click(browser, '#add-form button');
    await until(async () => (await order(browser)).includes('e2e_temporary'), 'Added channel missing');
    await click(browser, byLabel('Remove e2e_temporary'));
    await until(async () => (await order(browser)).length === 4, 'Removed channel retained');
    await click(browser, byLabel('Move bravo_demo up'));
    await until(async () => JSON.stringify(await order(browser)) === JSON.stringify(expectedOrder), 'Arrow rank not persisted into rendered state');
    await click(browser, byLabel('Enable alpha_demo'));
    await until(async () => (await browser.$('li[data-login="alpha_demo"]')).getAttribute('aria-disabled').then(value => value === 'true'), 'Enable change missing');
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
    await selectValue(browser, '#quality', '360p');
  });
  await test('two viewer handles render; pause/resume and Stop clean up', async () => {
    await click(browser, '#start');
    await until(async () => await visibleText(browser, '#run-status') === 'Monitoring', 'Start did not enter monitoring');
    const handles = await windows(browser, 3);
    for (const handle of handles.filter(handle => handle !== app.manager)) {
      await browser.switchToWindow(handle);
      assert.ok((await visibleText(browser, 'body')).length > 0, 'Native viewer document is blank');
    }
    await browser.switchToWindow(app.manager);
    await click(browser, '#pause');
    await until(async () => await visibleText(browser, '#run-status') === 'Automation paused', 'Pause did not render');
    await windows(browser, 3);
    await click(browser, '#start');
    await until(async () => await visibleText(browser, '#run-status') === 'Monitoring', 'Resume did not render');
    assert.equal(await app.screenshot('demo-two-windows'), 3, 'Screenshot capture must work for every native window');
    await click(browser, '#stop');
    await windows(browser, 1);
    await until(async () => (await browser.$$('#players > article')).length === 0, 'Stopped session cards retained');
  });
  if (extended) await test('real one-minute assignment timer rotates to the next live channel', async () => {
    await input(browser, '#limit', 1); await click(browser, 'h1');
    await input(browser, byLabel('Assignment timer minutes for bravo_demo'), 1);
    await click(browser, byLabel('Save timer for bravo_demo'));
    await until(async () => (await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('1 min assigned time'), 'One-minute timer not saved');
    const start = Date.now();
    await click(browser, '#start'); await windows(browser, 2);
    await until(async () => await visibleText(browser, '#players > article > strong') === 'bravo_demo', 'Expected first timed channel');
    await until(async () => await visibleText(browser, '#players > article > strong') === 'charlie_demo', 'Real elapsed timer did not rotate', 85000);
    assert.ok(Date.now() - start >= 55000, 'Rotation happened before the real assignment duration');
    await windows(browser, 2); await click(browser, '#stop'); await windows(browser, 1);
    await click(browser, byLabel('Set bravo_demo timer to 10 minutes'));
    await input(browser, '#limit', 2); await click(browser, 'h1');
  });
  await app.stop();
  browser = await app.start();
  await test('full process restart restores preferences without opening viewers', async () => {
    assert.deepEqual(await order(browser), expectedOrder);
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.equal(await (await browser.$('#limit')).getValue(), '2');
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
    await click(browser, '#start');
    const handles = await windows(browser, 3);
    const channels = [];
    for (const handle of handles.filter(handle => handle !== app.manager)) {
      await browser.switchToWindow(handle);
      assert.equal(await visibleText(browser, 'h1'), 'MPD E2E local viewer');
      channels.push(await visibleText(browser, '#channel'));
      await browser.execute(nonce => localStorage.setItem('mpd-e2e-test-nonce', nonce), app.runId);
    }
    assert.deepEqual(channels.sort(), ['alpha_fixture', 'bravo_fixture']);
    await browser.switchToWindow(app.manager);
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
    assert.equal(await visibleText(browser, '#run-status'), 'Monitoring');
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
  await test('native manager close exits the owned process', async () => {
    await app.nativeCommand({ action: 'close', label: app.manager });
    await until(() => app.child.exitCode !== null || app.child.signalCode !== null, 'Manager close did not exit the process');
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
    await until(async () => (await browser.$('#authorize')).isExisting(), 'Local fake activation UI missing');
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
    await windows(browser, 1);
  });
}
