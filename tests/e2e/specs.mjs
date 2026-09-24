import assert from 'node:assert/strict';
import { click, input, byLabel, order, until, visibleText, windows, delay } from './support.mjs';

const expectedOrder = ['bravo_demo', 'alpha_demo', 'charlie_demo', 'delta_demo'];
export async function demoSmoke(app, test, extended) {
  let browser = await app.start();
  await test('fresh isolated manager remains stopped', async () => {
    assert.equal(await visibleText(browser, '#run-status'), 'Stopped');
    assert.deepEqual(await order(browser), []);
    await windows(browser, 1);
  });
  await test('load demo, add/remove and arrow ranking through real UI', async () => {
    await (await browser.$('#source')).selectByAttribute('value', 'demo');
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
  await test('timer presets and settings save through production dispatch', async () => {
    await click(browser, byLabel('Set bravo_demo timer to 10 minutes'));
    await until(async () => (await visibleText(browser, 'li[data-login="bravo_demo"]')).includes('10 min assigned time'), 'Timer preset not saved');
    await input(browser, '#limit', 2); await click(browser, 'h1');
    await until(async () => (await visibleText(browser, '#session-count')).includes('/ 2'), 'Limit not saved');
    await (await browser.$('#quality')).selectByAttribute('value', '360p');
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
    await app.screenshot('demo-two-windows');
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
    await windows(browser, 1); await app.screenshot('restart-persistence');
  });
}
