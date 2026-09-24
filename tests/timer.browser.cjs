// Real Edge DOM, mocked native bridge. No native timing or Twitch credit claims.
const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
(async()=>{
 const browser=await chromium.launch({channel:'msedge',headless:true});let passed=0;
 async function check(name,test){const context=await browser.newContext({viewport:{width:860,height:1100}});const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));try{
  await page.route('**/*',route=>{const u=new URL(route.request().url());if(u.origin!=='https://mpd.test')return route.abort();const file=path.join(root,'ui',u.pathname==='/'?'index.html':u.pathname.slice(1));return fs.existsSync(file)?route.fulfill({path:file}):route.fulfill({status:404,body:''});});
  await page.addInitScript({content:fs.readFileSync(path.join(root,'tests/ui-fixture.js'),'utf8')+`
    window.__reads=0;window.__failSave=false;const original=__TAURI__.core.invoke;
    __TAURI__.core.invoke=async(command,args)=>{
      if(command==='get_state')__reads++;
      if(command==='dispatch'&&args.action.type==='set_timer'){
        if(__failSave)throw 'Simulated storage failure';
        __fixture.favorites.find(f=>f.login===args.action.login).watch_minutes=args.action.minutes;
        __fixture.settings.favorites.find(f=>f.login===args.action.login).watch_minutes=args.action.minutes;
      }
      return original(command,args);
    };
  `});
  await page.goto('https://mpd.test/');await page.waitForSelector('.timer-editor');await test(page);assert.deepEqual(errors,[]);console.log(`PASS ${name}`);passed++;
 }finally{await context.close();}}
 const timer=page=>page.getByRole('spinbutton',{name:'Assignment timer minutes for alpha_demo',exact:true});
 async function refresh(page){const reads=await page.evaluate(()=>__reads);await page.waitForFunction(reads=>__reads>reads,reads);}
 try{
  await check('Always default and explicit ten-minute save dispatch',async page=>{
   assert.equal(await timer(page).inputValue(),'');await page.getByRole('button',{name:'Set alpha_demo timer to 10 minutes',exact:true}).click();
   await page.waitForFunction(()=>__actions.some(a=>a.type==='set_timer'));assert.deepEqual(await page.evaluate(()=>__actions.filter(a=>a.type==='set_timer')),[{type:'set_timer',login:'alpha_demo',minutes:10}]);
   await page.waitForFunction(()=>document.querySelector('[data-login=alpha_demo] .channel-sub').textContent.includes('10 min assigned time'));
   await page.getByRole('button',{name:'Remove timer for alpha_demo',exact:true}).click();await page.waitForFunction(()=>__actions.at(-1).minutes===null);
  });
  await check('editing survives polling without implicit save and failed save keeps authoritative value',async page=>{
   await timer(page).fill('25');await page.evaluate(()=>{__fixture.favorites[0].presence='live';});await refresh(page);
   assert.equal(await timer(page).inputValue(),'25');assert.equal(await page.evaluate(()=>__actions.length),0);
   await page.evaluate(()=>__failSave=true);await page.getByRole('button',{name:'Save timer for alpha_demo',exact:true}).click();
   await page.waitForFunction(()=>document.getElementById('error-text').textContent.includes('storage failure'));
   assert(await page.locator('[data-login=alpha_demo] .channel-sub').textContent().then(t=>t.includes('Always')));
  });
  await check('invalid values never dispatch and controls fit minimum viewport',async page=>{
   for(const value of ['0','1441','1.5']){await timer(page).fill(value);await page.getByRole('button',{name:'Save timer for alpha_demo',exact:true}).click();}
   assert.equal(await page.evaluate(()=>__actions.length),0);
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   for(const width of [860,480]){await page.setViewportSize({width,height:1100});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}
  });
  await check('controller countdown updates without replacing an active drag row',async page=>{
   await page.evaluate(()=>{__fixture.players[0].timer={state:'counting',remaining_seconds:600};});await page.waitForFunction(()=>document.querySelector('.player-timer').textContent==='10:00 assigned time left');
   await page.evaluate(()=>{const row=document.querySelector('[data-login=alpha_demo]');window.__row=row;row.dispatchEvent(new DragEvent('dragstart',{bubbles:true,dataTransfer:new DataTransfer()}));__fixture.favorites[0].watch_minutes=10;__fixture.players[0].timer={state:'automation_paused',remaining_seconds:590};});
   await page.waitForFunction(()=>document.querySelector('.player-timer').textContent.includes('Automation paused'));
   assert(await page.evaluate(()=>__row===document.querySelector('[data-login=alpha_demo]')));
   await page.keyboard.press('Escape');
  });
 }finally{await browser.close();}
 console.log(`${passed} timer browser checks passed`);
})().catch(error=>{console.error(error);process.exitCode=1;});
