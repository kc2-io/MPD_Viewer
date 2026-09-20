// Real Edge drag/drop interactions against the manager UI with a mocked native bridge.
// This checks frontend behavior, not native WebView2 routing or preference persistence.
const {chromium} = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = process.env.MPD_TEST_ROOT || path.resolve(__dirname, '..');
const initial = ['alpha_demo', 'bravo_demo', 'charlie_demo', 'delta_demo'];
const fixture = fs.readFileSync(path.join(root, 'tests/ui-fixture.js'), 'utf8');

(async () => {
  const browser = await chromium.launch({channel: process.env.MPD_TEST_BROWSER || 'msedge', headless:true});
  let completed = 0;
  async function check(name, test, disabled = false) {
    const context = await browser.newContext({viewport:{width:1280,height:1100}});
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
      await page.route('**/*', route => {
        const url = new URL(route.request().url());
        if (url.origin !== 'https://mpd.test') return route.abort();
        const file = path.join(root, 'ui', url.pathname === '/' ? 'index.html' : url.pathname.slice(1));
        return fs.existsSync(file) ? route.fulfill({path:file}) : route.fulfill({status:404,body:''});
      });
      await page.addInitScript({content:fixture + `
        window.__reads = 0;
        window.__moveAttempts = [];
        window.__failMove = false;
        window.__deferMove = false;
        window.__deferRead = false;
        window.__failRead = false;
        window.__failReadsAfterMove = false;
        ${disabled ? "__fixture.favorites[0].enabled = false; __fixture.settings.favorites[0].enabled = false;" : ''}
        const originalInvoke = __TAURI__.core.invoke;
        __TAURI__.core.invoke = async (command, args) => {
          if (command === 'get_state') {
            __reads++;
            if (__failRead) throw new Error('Test state refresh failed');
            if (__deferRead) {
              __deferRead = false;
              const snapshot = await originalInvoke(command, args);
              return new Promise(resolve => {window.__finishRead = () => resolve(snapshot);});
            }
          }
          if (command === 'dispatch' && args.action.type === 'move') {
            __moveAttempts.push(structuredClone(args.action));
            if (__deferMove) await new Promise(resolve => {window.__finishMove = resolve;});
            if (__failMove) throw new Error('Test rank save failed');
            if (__failReadsAfterMove) __failRead = true;
          }
          return originalInvoke(command, args);
        };
      `});
      await page.goto('https://mpd.test/');
      await page.waitForFunction(() => document.querySelectorAll('.favorite').length === 4);
      await test(page);
      assert.deepEqual(errors, [], 'No uncaught JavaScript errors');
      completed++;
      console.log(`PASS ${name}`);
    } catch (error) {
      error.message = `${name}: ${error.message}`;
      throw error;
    } finally { await context.close(); }
  }
  const row = (page, login) => page.locator(`.favorite[data-login="${login}"]`);
  const order = page => page.locator('.favorite').evaluateAll(rows => rows.map(el => el.dataset.login));
  const attempts = page => page.evaluate(() => __moveAttempts);
  async function expectOrder(page, expected) {
    await page.waitForFunction(expected => JSON.stringify([...document.querySelectorAll('.favorite')].map(el => el.dataset.login)) === JSON.stringify(expected), expected);
    assert.deepEqual(await order(page), expected);
  }
  async function startMouseDrag(page, login, useText = false) {
    const source = row(page, login).locator(useText ? '.channel-info strong' : '.drag-grip');
    const bounds = await source.boundingBox();
    assert(bounds, 'Drag affordance is visible');
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.down();
    await page.mouse.move(bounds.x + bounds.width / 2 + 12, bounds.y + bounds.height / 2, {steps:5});
  }
  async function over(page, login, after) {
    const bounds = await row(page, login).boundingBox();
    const x = bounds.x + 100, y = bounds.y + bounds.height * (after ? .8 : .2);
    await page.mouse.move(x, y, {steps:12});
    // Chromium needs a second move at the target to emit dragover after dragenter.
    await page.mouse.move(x + 1, y);
  }
  async function drag(page, source, target, after, useText = false) {
    await startMouseDrag(page, source, useText);
    await over(page, target, after);
    await page.mouse.up();
  }
  async function eventDrop(page, source, target, after, external = false, control = false) {
    return page.evaluate(({source,target,after,external,control}) => {
      const sourceRow = document.querySelector(`.favorite[data-login="${source}"]`);
      const targetRow = document.querySelector(`.favorite[data-login="${target}"]`);
      const transfer = new DataTransfer();
      transfer.setData('text/plain', source);
      const start = new DragEvent('dragstart', {bubbles:true,cancelable:true,dataTransfer:transfer});
      if (!external) (control ? sourceRow.querySelector('button') : sourceRow).dispatchEvent(start);
      const bounds = targetRow.getBoundingClientRect();
      const options = {bubbles:true,cancelable:true,dataTransfer:transfer,clientX:bounds.x + 100,clientY:bounds.y + bounds.height * (after ? .8 : .2)};
      targetRow.dispatchEvent(new DragEvent('dragover', options));
      targetRow.dispatchEvent(new DragEvent('drop', options));
      if (!external) sourceRow.dispatchEvent(new DragEvent('dragend', {bubbles:true,dataTransfer:transfer}));
      return start.defaultPrevented;
    }, {source,target,after,external,control});
  }
  async function nextRefresh(page) {
    const reads = await page.evaluate(() => __reads);
    await page.waitForFunction(previous => __reads > previous, reads);
    // Let the async invoke result be consumed by render in the next microtask.
    await page.evaluate(() => Promise.resolve());
  }
  try {
    await check('mouse grip drag: first to last, rank feedback and arrow boundaries', async page => {
      assert.equal(await page.locator('#rank-status').getAttribute('aria-live'), 'polite');
      await drag(page, initial[0], initial[3], true);
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:3}]);
      assert(await page.getByRole('button', {name:`Move ${initial[1]} up`,exact:true}).isDisabled());
      assert(await page.getByRole('button', {name:`Move ${initial[0]} down`,exact:true}).isDisabled());
      assert((await page.locator('#rank-status').textContent()).trim(), 'A completed move is announced');
    });
    await check('mouse row drag: last to first', async page => {
      await drag(page, initial[3], initial[0], false, true);
      await expectOrder(page, [initial[3],initial[0],initial[1],initial[2]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[3],position:0}]);
    });
    await check('downward drop before target removes the source offset', async page => {
      await drag(page, initial[0], initial[3], false);
      await expectOrder(page, [initial[1],initial[2],initial[0],initial[3]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:2}]);
    });
    await check('upward drop after target uses the final rank', async page => {
      await drag(page, initial[3], initial[0], true);
      await expectOrder(page, [initial[0],initial[3],initial[1],initial[2]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[3],position:1}]);
    });
    await check('self and adjacent unchanged drops do not dispatch', async page => {
      for (const after of [false,true]) await eventDrop(page, initial[1], initial[1], after);
      await eventDrop(page, initial[1], initial[2], false);
      await eventDrop(page, initial[1], initial[0], true);
      assert.deepEqual(await attempts(page), []);
      assert.deepEqual(await order(page), initial);
    });
    await check('Escape cancels an actual mouse drag', async page => {
      await startMouseDrag(page, initial[0]);
      await over(page, initial[3], true);
      await page.keyboard.press('Escape');
      await page.mouse.up();
      assert.deepEqual(await attempts(page), []);
      assert.deepEqual(await order(page), initial);
      await drag(page, initial[3], initial[0], false);
      await expectOrder(page, [initial[3],initial[0],initial[1],initial[2]]);
    });
    await check('external text drops cannot reorder a matching channel', async page => {
      await eventDrop(page, initial[0], initial[3], true, true);
      assert.deepEqual(await attempts(page), []);
      assert.deepEqual(await order(page), initial);
    });
    await check('row buttons reject drag initiation and arrows retain keyboard operation', async page => {
      assert(await eventDrop(page, initial[1], initial[3], true, false, true), 'Control dragstart is cancelled');
      assert.deepEqual(await attempts(page), []);
      await page.getByRole('button', {name:`Move ${initial[1]} up`,exact:true}).focus();
      await page.keyboard.press('Enter');
      await expectOrder(page, [initial[1],initial[0],initial[2],initial[3]]);
      await page.getByRole('button', {name:`Move ${initial[1]} down`,exact:true}).focus();
      await page.keyboard.press('Space');
      await expectOrder(page, initial);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[1],position:0},{type:'move',login:initial[1],position:1}]);
    });
    await check('presence refresh keeps the drag DOM stable and drop remains usable', async page => {
      await page.evaluate(() => {window.__heldRow = document.querySelector('.favorite');});
      await startMouseDrag(page, initial[0]);
      await over(page, initial[3], true);
      await page.evaluate(() => {__fixture.favorites[0].presence = 'live';});
      await nextRefresh(page);
      assert(await page.evaluate(() => __heldRow === document.querySelector('.favorite')), 'Refresh must not replace the drag source');
      await page.mouse.up();
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:3}]);
    });
    await check('changed authoritative order cancels a stale drag', async page => {
      await startMouseDrag(page, initial[0]);
      await over(page, initial[3], true);
      await page.evaluate(() => {__fixture.favorites.reverse();__fixture.settings.favorites.reverse();});
      await nextRefresh(page);
      await page.mouse.up();
      await expectOrder(page, [...initial].reverse());
      assert.deepEqual(await attempts(page), []);
      assert((await page.locator('#rank-status').textContent()).trim(), 'Stale order cancellation is announced');
    });
    await check('pending save retains order and ignores a second reorder', async page => {
      await page.evaluate(() => {__deferMove = true;window.__heldRow = document.querySelector('.favorite');});
      await drag(page, initial[0], initial[3], true);
      await page.waitForFunction(() => typeof __finishMove === 'function');
      await page.evaluate(() => {__fixture.favorites[0].presence = 'live';});
      await nextRefresh(page);
      assert.deepEqual(await order(page), initial, 'No optimistic rank change before native success');
      assert(await page.evaluate(() => __heldRow === document.querySelector('.favorite')), 'Pending save must not replace rows');
      await eventDrop(page, initial[3], initial[0], false);
      const arrow = page.getByRole('button', {name:`Move ${initial[1]} up`,exact:true});
      if (!await arrow.isDisabled()) await arrow.click();
      assert.equal((await attempts(page)).length, 1);
      await page.evaluate(() => {__deferMove = false;__finishMove();});
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
    });
    await check('save waits for a fresh snapshot after an older in-flight poll', async page => {
      await page.evaluate(() => {__deferRead = true;});
      await page.waitForFunction(() => typeof __finishRead === 'function');
      await drag(page, initial[0], initial[3], true);
      await page.waitForFunction(() => __fixture.favorites[3].login === 'alpha_demo');
      assert.deepEqual(await order(page), initial, 'An unresolved old poll must not finish the move');
      await page.evaluate(() => __finishRead());
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:3}]);
      assert((await page.locator('#rank-status').textContent()).trim(), 'Fresh completion is announced');
    });
    await check('failed native move preserves order, reports failure and permits retry', async page => {
      await page.evaluate(() => {__failMove = true;});
      await drag(page, initial[0], initial[3], true);
      await page.waitForFunction(() => !document.getElementById('error').hidden);
      assert.match(await page.locator('#error-text').textContent(), /Test rank save failed/);
      assert.deepEqual(await order(page), initial);
      assert((await page.locator('#rank-status').textContent()).trim(), 'Failure is announced');
      await page.evaluate(() => {__failMove = false;});
      await drag(page, initial[0], initial[3], true);
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      assert.equal((await attempts(page)).length, 2);
    });
    await check('saved move with failed refresh blocks reordering until authoritative recovery', async page => {
      await page.evaluate(() => {__failReadsAfterMove = true;});
      await drag(page, initial[0], initial[3], true);
      await page.waitForFunction(() => document.getElementById('error-text').textContent.includes('Test state refresh failed'));
      assert.deepEqual(await page.evaluate(() => __fixture.favorites.map(f => f.login)), [initial[1],initial[2],initial[3],initial[0]], 'Native move already succeeded');
      assert.deepEqual(await order(page), initial, 'UI still awaits its authoritative snapshot');
      assert.match(await page.locator('#rank-status').textContent(), /saved/i);
      assert.match(await page.locator('#rank-status').textContent(), /refresh/i);
      assert.doesNotMatch(await page.locator('#rank-status').textContent(), /could not save/i);
      await eventDrop(page, initial[3], initial[0], false);
      const arrow = page.getByRole('button', {name:`Move ${initial[1]} up`,exact:true});
      if (!await arrow.isDisabled()) await arrow.click();
      assert.equal((await attempts(page)).length, 1, 'Stale ranks cannot dispatch another move');
      await page.evaluate(() => {__failRead = false;__failReadsAfterMove = false;});
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      await page.getByRole('button', {name:`Move ${initial[0]} up`,exact:true}).click();
      await expectOrder(page, [initial[1],initial[2],initial[0],initial[3]]);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:3},{type:'move',login:initial[0],position:2}]);
    });
    await check('disabled channels remain reorderable without changing enabled state', async page => {
      await drag(page, initial[0], initial[3], true);
      await expectOrder(page, [initial[1],initial[2],initial[3],initial[0]]);
      assert.equal(await row(page, initial[0]).locator('input[type=checkbox]').isChecked(), false);
      assert.equal(await page.evaluate(() => __fixture.settings.favorites.find(f => f.login === 'alpha_demo').enabled), false);
      assert.deepEqual(await attempts(page), [{type:'move',login:initial[0],position:3}]);
    }, true);
    console.log(`PASS all ${completed} ranking browser cases (mocked native bridge; Edge ${browser.version()})`);
  } finally { await browser.close(); }
})().catch(error => {console.error(error);process.exitCode = 1;});
