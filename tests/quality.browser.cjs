// Real Edge manager UI with mocked native settings; not native preference-storage evidence.
const {chromium}=require('playwright');
const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
const root=process.env.MPD_TEST_ROOT||path.resolve(__dirname,'..');
const fixture=fs.readFileSync(path.join(root,'tests/ui-fixture.js'),'utf8');
(async()=>{
  const browser=await chromium.launch({channel:process.env.MPD_TEST_BROWSER||'msedge',headless:true});
  let passed=0;
  async function check(name,test){
    const context=await browser.newContext({viewport:{width:1280,height:1100}});
    const page=await context.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
    try{
      await page.route('**/*',route=>{
        const url=new URL(route.request().url());if(url.origin!=='https://mpd.test')return route.abort();
        const file=path.join(root,'ui',url.pathname==='/'?'index.html':url.pathname.slice(1));
        return fs.existsSync(file)?route.fulfill({path:file}):route.fulfill({status:404,body:''});
      });
      await page.addInitScript({content:fixture+`
        __fixture.settings.preferred_quality=sessionStorage.getItem('mock-quality')||'auto';
        window.__reads=0;const original=__TAURI__.core.invoke;
        __TAURI__.core.invoke=async(command,args)=>{
          if(command==='get_state')__reads++;
          if(command==='dispatch'&&args.action.type==='set_quality'){
            __fixture.settings.preferred_quality=args.action.quality;
            sessionStorage.setItem('mock-quality',args.action.quality);
          }
          return original(command,args);
        };
      `});
      await page.goto('https://mpd.test/');await page.waitForSelector('#quality');
      await test(page);assert.deepEqual(errors,[]);passed++;console.log(`PASS ${name}`);
    }catch(error){error.message=`${name}: ${error.message}`;throw error;}finally{await context.close();}
  }
  async function checkWrapper(name,variants,demo,test){
    const context=await browser.newContext({viewport:{width:1000,height:750}});
    const page=await context.newPage();const errors=[];let sdkRequests=0,mainNavigations=0;
    page.on('pageerror',error=>errors.push(error.message));
    page.on('framenavigated',frame=>{if(frame===page.mainFrame())mainNavigations++;});
    const sdkSource=`
      window.__sdk={instances:0,plays:0,pauses:0,qualityCalls:[]};
      class Player {
        constructor(id,options){__sdk.instances++;__sdk.options=options;__sdk.player=this;this.events={};this.paused=true;this.quality='auto';this.volume=0;this.muted=true;}
        addEventListener(event,callback){(this.events[event]??=[]).push(callback);}
        emit(event){for(const callback of this.events[event]??[])callback();}
        getQualities(){return window.__variants;}
        getQuality(){return this.quality;}
        setQuality(value){__sdk.qualityCalls.push(value);this.quality=value;}
        isPaused(){return this.paused;}
        play(){__sdk.plays++;this.paused=false;this.emit(Player.PLAY);this.emit(Player.PLAYING);}
        pause(){__sdk.pauses++;this.paused=true;this.emit(Player.PAUSE);}
        getVolume(){return this.volume;}
        getMuted(){return this.muted;}
        setVolume(value){this.volume=value;}
        setMuted(value){this.muted=value;}
      }
      for(const event of ['READY','PLAY','PLAYING','PAUSE','PLAYBACK_BLOCKED','OFFLINE','ENDED'])Player[event]=event;
      window.Twitch={Player};
    `;
    try{
      await page.route('**/*',route=>{
        const url=new URL(route.request().url());
        if(url.href==='https://player.twitch.tv/js/embed/v1.js'){
          sdkRequests++;return route.fulfill({contentType:'application/javascript',body:sdkSource});
        }
        if(url.origin==='https://www.twitch.tv'&&url.pathname.startsWith('/embed/'))return route.fulfill({contentType:'text/html',body:'<!doctype html><title>Mock chat</title>'});
        if(url.origin!=='https://wrapper.test')return route.abort();
        const file=path.join(root,'player-wrapper',url.pathname==='/'?'index.html':url.pathname.slice(1));
        return fs.existsSync(file)?route.fulfill({path:file}):route.fulfill({status:404,body:''});
      });
      await page.addInitScript({content:`
        window.__variants=${JSON.stringify(variants)};
        window.__reports=[];window.__cspViolations=[];
        document.addEventListener('securitypolicyviolation',event=>__cspViolations.push(event.violatedDirective));
        window.__TAURI__={core:{invoke:async(command,args)=>{if(command==='player_report')__reports.push(args.report);}}};
      `});
      await page.goto(`https://wrapper.test/index.html#channel=alpha&session=7&quality=180p&volume=25&muted=false&demo=${demo}`);
      await page.waitForFunction(()=>typeof window.mpdSetQuality==='function');
      assert.match(await page.locator('meta[http-equiv="Content-Security-Policy"]').getAttribute('content'),/script-src 'self' https:\/\/player\.twitch\.tv/);
      if(!demo){
        await page.waitForFunction(()=>window.__sdk?.instances===1);
        await page.evaluate(()=>__sdk.player.emit(Twitch.Player.READY));
        await page.waitForFunction(()=>document.getElementById('status').textContent==='playing');
      }
      await test(page);
      assert.equal(mainNavigations,1,'Quality updates never reload or navigate the wrapper');
      assert.equal(sdkRequests,demo?0:1,'The SDK is loaded once for real playback and never for demo');
      assert.deepEqual(await page.evaluate(()=>__cspViolations),[],'Actual wrapper CSP remains effective');
      assert.deepEqual(errors,[]);passed++;console.log(`PASS ${name}`);
    }catch(error){error.message=`${name}: ${error.message}`;throw error;}finally{await context.close();}
  }
  const expectValue=(page,value)=>page.waitForFunction(value=>document.getElementById('quality').value===value,value);
  async function refreshed(page){const reads=await page.evaluate(()=>__reads);await page.waitForFunction(reads=>__reads>reads,reads);await page.evaluate(()=>Promise.resolve());}
  try{
    await check('quality defaults to auto and offers all supported choices',async page=>{
      await expectValue(page,'auto');
      assert.deepEqual(await page.locator('#quality option').evaluateAll(items=>items.map(item=>item.value)),['auto','source','160p','180p','240p','360p','480p','720p','1080p','1440p','2160p']);
      assert.equal(await page.locator('label[for="quality"]').count(),1);
    });
    await check('180p selection dispatches exact preference and reconstructs from backend setting',async page=>{
      await page.locator('#quality').selectOption('180p');
      await page.waitForFunction(()=>__actions.some(action=>action.type==='set_quality'));
      assert.deepEqual(await page.evaluate(()=>__actions.filter(action=>action.type==='set_quality')),[{type:'set_quality',quality:'180p'}]);
      await page.reload();await expectValue(page,'180p');
      assert.deepEqual(await page.evaluate(()=>__actions),[],'Rendering saved preference does not redispatch it');
    });
    await check('polling preserves focused selection and applies backend preference after blur',async page=>{
      await expectValue(page,'auto');await page.locator('#quality').focus();
      await page.evaluate(()=>{__fixture.settings.preferred_quality='1080p';});await refreshed(page);
      assert.equal(await page.locator('#quality').inputValue(),'auto');
      await page.locator('#quality').evaluate(element=>element.blur());await expectValue(page,'1080p');
      assert.deepEqual(await page.evaluate(()=>__actions),[]);
    });
    await check('quality control and label fit manager minimum width',async page=>{
      await page.setViewportSize({width:860,height:1100});await page.locator('#quality').selectOption('2160p');
      const bounds=await page.locator('#quality').evaluate(element=>{
        const control=element.getBoundingClientRect(),panel=element.closest('.panel').getBoundingClientRect();
        const label=document.querySelector('label[for="quality"]').getBoundingClientRect();
        return {control:{left:control.left,right:control.right},panel:{left:panel.left,right:panel.right},label:{left:label.left,right:label.right}};
      });
      assert(bounds.control.left>=bounds.panel.left&&bounds.control.right<=bounds.panel.right);
      assert(bounds.label.left>=bounds.panel.left&&bounds.label.right<=bounds.panel.right);
      assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
      if(process.env.MPD_QUALITY_SCREENSHOT)await page.screenshot({path:process.env.MPD_QUALITY_SCREENSHOT,fullPage:true});
    });
    await checkWrapper('bundled wrapper applies fragment preference, changes existing player, and preserves manual pause',['auto','360p30','720p30','chunked'],false,async page=>{
      assert.deepEqual(await page.evaluate(()=>__sdk.qualityCalls),['360p30']);
      assert.equal(await page.evaluate(()=>__sdk.plays),1,'Only initial READY playback was requested');
      await page.evaluate(()=>mpdSetQuality('720p'));
      assert.deepEqual(await page.evaluate(()=>__sdk.qualityCalls),['360p30','720p30']);
      assert.equal(await page.evaluate(()=>__sdk.instances),1);
      assert.equal(await page.evaluate(()=>__sdk.plays),1);
      await page.evaluate(()=>{__sdk.player.pause();mpdSetQuality('180p');document.dispatchEvent(new Event('visibilitychange'));});
      assert.deepEqual(await page.evaluate(()=>__sdk.qualityCalls),['360p30','720p30']);
      assert.equal(await page.evaluate(()=>__sdk.player.isPaused()),true);
      assert.equal(await page.evaluate(()=>__sdk.plays),1,'Changing quality never resumes a paused player');
      assert.equal(await page.locator('#status').textContent(),'paused');
      await page.locator('#try-play').click();
      await page.waitForFunction(()=>__sdk.qualityCalls.at(-1)==='360p30');
      assert.equal(await page.evaluate(()=>__sdk.plays),2,'Only the explicit user action requests resume');
      assert.equal(await page.evaluate(()=>__sdk.instances),1);
      assert.equal(await page.locator('#status').textContent(),'playing');
    });
    await checkWrapper('bundled wrapper falls back to advertised source-only rendition',['auto','chunked'],false,async page=>{
      assert.deepEqual(await page.evaluate(()=>__sdk.qualityCalls),['chunked']);
      assert.equal(await page.evaluate(()=>__sdk.instances),1);
    });
    await checkWrapper('bundled demo never constructs or contacts the Twitch SDK',[],true,async page=>{
      await page.waitForFunction(()=>document.getElementById('status').textContent==='Simulated · playing');
      await page.evaluate(()=>{mpdSetQuality('720p');document.dispatchEvent(new Event('visibilitychange'));});
      assert.equal(await page.evaluate(()=>typeof window.__sdk),'undefined');
      assert.equal(await page.locator('iframe').count(),0);
      assert.equal(await page.locator('#status').textContent(),'Simulated · playing');
      assert(await page.locator('#demo').isVisible());
    });
    console.log(`${passed} quality browser cases passed (${browser.version()}).`);
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
