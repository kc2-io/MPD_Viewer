// Real Edge UI interactions with mocked native state. No Twitch traffic or native-title proof.
const {chromium} = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = process.env.MPD_TEST_ROOT || path.resolve(__dirname, '..');
const fixture = fs.readFileSync(path.join(root, 'tests/ui-fixture.js'), 'utf8');
const names = ['alpha_demo','bravo_demo','charlie_demo','delta_demo'];

(async () => {
  const browser = await chromium.launch({channel:process.env.MPD_TEST_BROWSER || 'msedge',headless:true});
  let completed = 0;
  const row = (page,login) => page.locator(`.favorite[data-login="${login}"]`);
  const label = (page,login) => row(page,login).locator('.channel-sub .viewer-count');
  async function expectLabel(page,login,text) {
    await page.waitForFunction(({login,text}) => {
      const element = document.querySelector(`.favorite[data-login="${login}"] .channel-sub .viewer-count`);
      return text === null ? element === null : element?.textContent === text;
    }, {login,text});
    if (text === null) assert.equal(await label(page,login).count(),0);
    else assert.equal(await label(page,login).textContent(),text);
  }
  async function nextRefresh(page) {
    const reads = await page.evaluate(() => __reads);
    await page.waitForFunction(reads => __reads > reads,reads);
    await page.evaluate(() => Promise.resolve());
  }
  async function check(name,test) {
    const context = await browser.newContext({viewport:{width:1280,height:1100}});
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror',error => errors.push(error.message));
    try {
      await page.route('**/*',route => {
        const url = new URL(route.request().url());
        if (url.origin !== 'https://mpd.test') return route.abort();
        const file = path.join(root,'ui',url.pathname === '/' ? 'index.html' : url.pathname.slice(1));
        return fs.existsSync(file) ? route.fulfill({path:file}) : route.fulfill({status:404,body:''});
      });
      await page.addInitScript({content:fixture + `
        __fixture.settings.demo = false;
        __fixture.favorites.forEach((favorite,index) => {
          favorite.presence = 'live';
          favorite.viewer_count = index < 3 ? {count:[0,1,1234][index],stale:false} : null;
        });
        window.__reads = 0;
        const originalInvoke = __TAURI__.core.invoke;
        __TAURI__.core.invoke = async (command,args) => {
          if (command === 'get_state') __reads++;
          return originalInvoke(command,args);
        };
      `});
      await page.goto('https://mpd.test/');
      await page.waitForFunction(() => document.querySelectorAll('.favorite').length === 4);
      await test(page);
      assert.deepEqual(errors,[],'No uncaught JavaScript errors');
      completed++;
      console.log(`PASS ${name}`);
    } catch (error) { error.message = `${name}: ${error.message}`; throw error; }
    finally { await context.close(); }
  }
  try {
    await check('channel rows show live zero, singular and grouped viewer counts',async page => {
      await expectLabel(page,names[0],'0 viewers');
      await expectLabel(page,names[1],'1 viewer');
      await expectLabel(page,names[2],'1,234 viewers');
      await expectLabel(page,names[3],null);
    });
    await check('offline and unknown observations hide absent counts',async page => {
      await page.evaluate(() => {
        __fixture.favorites[0].presence = 'offline';
        __fixture.favorites[1].presence = 'unknown';
        __fixture.favorites[0].viewer_count = null;
        __fixture.favorites[1].viewer_count = null;
      });
      await expectLabel(page,names[0],null);
      await expectLabel(page,names[1],null);
      assert.match(await row(page,names[0]).locator('.channel-sub').textContent(),/OFFLINE/);
      assert.match(await row(page,names[1]).locator('.channel-sub').textContent(),/UNKNOWN/);
    });
    await check('polling updates counts and marks retained observations stale',async page => {
      await page.evaluate(() => {__fixture.favorites[2].viewer_count = {count:5678,stale:false};});
      await expectLabel(page,names[2],'5,678 viewers');
      await page.evaluate(() => {
        __fixture.favorites[2].presence = 'unknown';
        __fixture.favorites[2].viewer_count.stale = true;
      });
      await expectLabel(page,names[2],'5,678 viewers (stale)');
      await page.evaluate(() => {
        __fixture.favorites[2].presence = 'live';
        __fixture.favorites[2].viewer_count = {count:1,stale:false};
      });
      await expectLabel(page,names[2],'1 viewer');
    });
    await check('demo mode suppresses even injected real-looking counts',async page => {
      await expectLabel(page,names[2],'1,234 viewers');
      await page.evaluate(() => {__fixture.settings.demo = true;});
      await page.waitForFunction(() => document.querySelectorAll('.viewer-count').length === 0);
      assert.equal(await page.locator('.viewer-count').count(),0);
      await page.evaluate(() => {__fixture.settings.demo = false;});
      await expectLabel(page,names[2],'1,234 viewers');
    });
    await check('viewer-count refresh preserves an active mouse drag and displays the new count after drop',async page => {
      await expectLabel(page,names[0],'0 viewers');
      await page.evaluate(() => {window.__heldRow = document.querySelector('.favorite');});
      const source = await row(page,names[0]).locator('.drag-grip').boundingBox();
      assert(source);
      await page.mouse.move(source.x + source.width / 2,source.y + source.height / 2);
      await page.mouse.down();
      await page.mouse.move(source.x + source.width / 2 + 12,source.y + source.height / 2,{steps:5});
      const target = await row(page,names[3]).boundingBox();
      assert(target);
      await page.mouse.move(target.x + 100,target.y + target.height * .8,{steps:12});
      await page.mouse.move(target.x + 101,target.y + target.height * .8);
      assert.equal(await row(page,names[0]).evaluate(el => el.classList.contains('drag-source')),true);
      await page.evaluate(() => {__fixture.favorites[0].viewer_count = {count:42,stale:false};});
      await nextRefresh(page);
      assert(await page.evaluate(() => __heldRow === document.querySelector('.favorite')),'Polling must not replace the dragged row');
      assert.equal(await label(page,names[0]).textContent(),'0 viewers','Count waits for the drag to finish');
      await page.mouse.up();
      await page.waitForFunction(() => document.querySelector('.favorite:last-child')?.dataset.login === 'alpha_demo');
      await expectLabel(page,names[0],'42 viewers');
      assert.deepEqual(await page.evaluate(() => __actions.filter(a => a.type === 'move')),[{type:'move',login:names[0],position:3}]);
      await page.getByRole('button',{name:`Move ${names[0]} up`,exact:true}).click();
      await page.waitForFunction(() => document.querySelectorAll('.favorite')[2]?.dataset.login === 'alpha_demo');
      await expectLabel(page,names[0],'42 viewers');
    });
    await check('minimum width keeps maximum and stale viewer labels clear of row controls',async page => {
      await page.setViewportSize({width:860,height:1100});
      await page.evaluate(() => {
        __fixture.favorites.forEach((favorite,index) => {
          favorite.viewer_count = {count:4294967295,stale:index > 0};
          favorite.presence = index > 0 ? 'unknown' : 'live';
          favorite.skipped = index > 1;
          favorite.open_error = index === 3 ? 'Test window could not open' : null;
        });
        __fixture.players.push({login:'alpha_demo',session:99,closing:false,state:'playing',report_age_seconds:0,visible:true,volume:.25,muted:false});
      });
      await expectLabel(page,names[0],'4,294,967,295 viewers');
      await expectLabel(page,names[3],'4,294,967,295 viewers (stale)');
      await page.evaluate(() => document.fonts.ready);
      const geometry = await page.locator('.favorite').evaluateAll(rows => rows.map(row => {
        const box = element => {
          const r = element.getBoundingClientRect();
          return {left:r.left,right:r.right,top:r.top,bottom:r.bottom};
        };
        return {login:row.dataset.login,row:box(row),controls:box(row.querySelector('.row-controls')),
          labels:[...row.querySelectorAll('.channel-sub span')].map(label => {
            const range = document.createRange(); range.selectNodeContents(label);
            return {text:label.textContent,...box(range)};
          })};
      }));
      console.log('Minimum-width label geometry:',JSON.stringify(geometry));
      if (process.env.MPD_VIEWER_COUNT_SCREENSHOT) await page.screenshot({path:process.env.MPD_VIEWER_COUNT_SCREENSHOT,fullPage:true});
      for (const item of geometry) {
        for (const label of item.labels) {
          assert(label.right <= item.controls.left - 2,`${item.login} ${label.text} ends at ${label.right}, controls begin at ${item.controls.left}`);
          assert(label.left >= item.row.left && label.bottom <= item.row.bottom,`${item.login} ${label.text} fits its row`);
        }
        assert(item.controls.right <= item.row.right + 1,`${item.login} controls fit its row`);
      }
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),'No page horizontal overflow at 860px');
    });
    console.log(`${completed} viewer-count browser cases passed (${browser.version()}).`);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
