// Optional integration check: NODE_PATH must resolve Playwright; uses installed Edge.
// Network responses are mocked. This does not verify Twitch login or posting.
const {chromium} = require('playwright');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const chat = fs.readFileSync(path.join(root,'player-wrapper/chat.js'),'utf8');
const css = fs.readFileSync(path.join(root,'player-wrapper/chat.css'),'utf8');
const sdk = `window.instances=[]; class Player {
 constructor(id,options){this.options=options;this.paused=true;this.volume=.25;this.muted=true;this.plays=0;instances.push(this);const f=document.createElement('iframe');f.src='https://player.twitch.tv/?channel='+options.channel;f.width='100%';f.height='100%';document.getElementById(id).append(f);}
 addEventListener(){} setVolume(v){this.volume=v} setMuted(v){this.muted=v} getVolume(){return this.volume} getMuted(){return this.muted} play(){this.plays++;this.paused=false} pause(){this.paused=true}
} window.Twitch={Player};`;
(async()=>{
 const server=http.createServer((req,res)=>{
  const name=new URL(req.url,'http://localhost').pathname.slice(1)||'index.html';
  if(!['index.html','player.js','quality.js','style.css','chat.js','chat.css'].includes(name)){res.writeHead(404).end();return;}
  res.setHeader('Content-Type',name.endsWith('.js')?'text/javascript':name.endsWith('.css')?'text/css':'text/html');
  res.end(fs.readFileSync(path.join(root,'player-wrapper',name)));
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({channel:'msedge',headless:true});
 try {
  for(const mode of ['hosted','bundled','demo']){
   const context=await browser.newContext({viewport:{width:1180,height:720}});
   const page=await context.newPage(); const violations=[]; let twitchRequests=0;
   await page.exposeFunction('violation',e=>violations.push(e));
   await page.addInitScript(()=>document.addEventListener('securitypolicyviolation',e=>window.violation(e.blockedURI)));
   await page.route('https://**/*',route=>{
    const url=route.request().url();
    if(url.includes('twitch.tv')) twitchRequests++;
    if(url==='https://player.twitch.tv/js/embed/v1.js')return route.fulfill({contentType:'text/javascript',body:sdk});
    if(url.startsWith('https://parent.mpdviewer.com/'))return route.fulfill({contentType:'text/html',body:fs.readFileSync(path.join(root,'web/parent.mpdviewer.com/index.html'),'utf8')});
    return route.fulfill({contentType:'text/html',body:'<!doctype html><p>Mock embedded content</p>'});
   });
   if(mode==='hosted'){
    await page.addInitScript({content:`(${chat})(${JSON.stringify({channel:'alpha',mode:'hosted',origin:'https://parent.mpdviewer.com',path:'/',demo:false,css})});`});
    await page.goto('https://parent.mpdviewer.com/?channel=alpha&active=__mpd-native-pending__&pauseInactive=false');
   }else await page.goto(`http://localhost:${server.address().port}/index.html#channel=alpha&session=7&demo=${mode==='demo'}&volume=25&muted=true`);
   await page.locator('#mpd-chat-toggle').waitFor();
   const videoSelector=mode==='hosted'?'#grid':mode==='demo'?'#demo':'#video';
   const size=await page.locator(videoSelector).boundingBox();
   assert.ok(size.width>=400&&size.height>=300,JSON.stringify(size));
   await page.evaluate(()=>{
    const video=document.querySelector('.mpd-chat-video iframe'); const chat=document.querySelector('#mpd-chat-panel iframe');
    window.beforeChat={video,parent:video?.parentNode,grid:document.querySelector('.mpd-chat-video'),gridParent:document.querySelector('.mpd-chat-video').parentNode,chat,videoWindow:video?.contentWindow,chatWindow:chat?.contentWindow,player:window.instances?.[0]};
    if(window.beforeChat.player)window.beforeChat.player.pause();
   });
   let navigations=0; page.on('framenavigated',()=>navigations++);
   for(let i=0;i<6;i++)await page.locator('#mpd-chat-toggle').click();
   await page.setViewportSize({width:430,height:480});
   await page.waitForTimeout(150);
   const narrow=await page.locator(videoSelector).boundingBox(), panel=await page.locator('#mpd-chat-panel').boundingBox();
   assert.ok(narrow.width>=400&&narrow.height>=300); assert.ok(panel.y>=narrow.y+narrow.height-1);
   assert.equal(await page.evaluate(()=>{const b=beforeChat;return b.grid.parentNode===b.gridParent&&(!b.video||(b.video.parentNode===b.parent&&b.video.contentWindow===b.videoWindow))&&(!b.chat||b.chat.contentWindow===b.chatWindow)&&(!b.player||(instances.length===1&&instances[0]===b.player&&b.player.paused&&b.player.plays===0&&b.player.volume===.25&&b.player.muted));}),true);
   assert.equal(navigations,0); assert.deepEqual(violations,[]);
   if(mode==='demo'){assert.equal(twitchRequests,0);assert.equal(await page.locator('iframe').count(),0);}
   else {
    await page.locator('#mpd-chat-panel iframe').evaluate(frame=>frame.dispatchEvent(new Event('error')));
    assert.match(await page.locator('#mpd-chat-status').textContent(),/could not load/);
    assert.equal(await page.evaluate(()=>instances[0].paused),true);
   }
   console.log(`PASS ${mode}: real CSP, desktop/stacked geometry, retained contexts, no resume, isolated chat failure/demo network`);
   await context.close();
  }
 } finally {await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
