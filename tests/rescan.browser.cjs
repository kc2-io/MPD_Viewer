// Real Edge DOM with mocked native settings. Native scheduling is covered by Rust tests.
const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
(async()=>{
 const launch={headless:true};if(process.env.MPD_TEST_CHROMIUM)launch.executablePath=process.env.MPD_TEST_CHROMIUM;else launch.channel=process.env.MPD_TEST_BROWSER||'msedge';
 const browser=await chromium.launch(launch);
 const context=await browser.newContext({viewport:{width:1280,height:1100}});const page=await context.newPage();const errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 try{
  await page.route('**/*',route=>{const u=new URL(route.request().url());if(u.origin!=='https://mpd.test')return route.abort();const file=path.join(root,'ui',u.pathname==='/'?'index.html':u.pathname.slice(1));return fs.existsSync(file)?route.fulfill({path:file}):route.fulfill({status:404,body:''});});
  await page.addInitScript({content:fs.readFileSync(path.join(root,'tests/ui-fixture.js'),'utf8')+'\nwindow.__reads=0;const original=__TAURI__.core.invoke;__TAURI__.core.invoke=async(command,args)=>{if(command==="get_state")__reads++;return original(command,args);};'});
  await page.goto('https://mpd.test/');await page.waitForSelector('.player-card');
  const rescan=page.getByRole('spinbutton',{name:'Live-status rescan interval (minutes)',exact:true});
  assert.equal(await rescan.getAttribute('aria-describedby'),'rescan-hint');
  assert.equal(await page.locator('#monitor-heading').textContent(),'Demo source');
  assert((await page.locator('.workspace-panel').boundingBox()).y < (await page.locator('.metrics').boundingBox()).y);
  await page.screenshot({path:path.join(root,'docs/ui-preview.png'),fullPage:true});
  await page.evaluate(()=>{__fixture.settings.demo=false;});
  await page.waitForFunction(()=>document.querySelector('#monitor-heading').textContent==='Every 1 min');
  await rescan.fill('12');const reads=await page.evaluate(()=>__reads);await page.waitForFunction(reads=>__reads>reads,reads);
  assert.equal(await rescan.inputValue(),'12','Focused partial edit survives polling');
  assert.equal(await page.evaluate(()=>__actions.filter(a=>a.type==='set_rescan').length),0);
  await rescan.press('Enter');await page.waitForFunction(()=>__actions.some(a=>a.type==='set_rescan'));
  assert.deepEqual(await page.evaluate(()=>__actions.filter(a=>a.type==='set_rescan')),[{type:'set_rescan',minutes:12}]);
  await page.evaluate(()=>{__fixture.error='Existing Twitch polling error.';});
  await rescan.fill('0');await rescan.press('Enter');await page.waitForFunction(()=>!document.getElementById('error').hidden);
  assert.equal(await page.evaluate(()=>__actions.filter(a=>a.type==='set_rescan').length),1,'Invalid value never dispatches');
  assert.equal(await rescan.inputValue(),'0');assert(await rescan.evaluate(element=>element===document.activeElement));
  const readsAfterError=await page.evaluate(()=>__reads);await page.waitForFunction(reads=>__reads>reads,readsAfterError);
  assert.match(await page.locator('#error-text').textContent(),/rescan interval/,'Local validation remains visible over an older poll error');
  for(const width of [860,729,721,480]){await page.setViewportSize({width,height:1100});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}
  assert.deepEqual(errors,[]);console.log('PASS top player status and accessible persisted rescan interaction');
 }finally{await context.close();await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
