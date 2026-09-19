"""In-memory browser DOM tests of UI/wrapper logic, with mocked native IPC and Twitch SDK.
These DO NOT validate Rust compilation, native WebViews, CSP/module loading, or actual Twitch playback.
Run with Python + Playwright installed. Browser defaults to Playwright's Chromium;
MPD_TEST_CHROMIUM can select an existing browser binary.
"""
import json
import os
from pathlib import Path
import re
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
FAKE_SDK=r'''
window.__sdkCalls=[];
class FakePlayer {
  constructor(id,options){window.__sdk=this;window.__sdkOptions=options;this.events={};this.volume=1;this.muted=true;}
  addEventListener(name,callback){(this.events[name]??=[]).push(callback);}
  fire(name){for(const cb of this.events[name]??[])cb();}
  setVolume(v){this.volume=v;window.__sdkCalls.push(['volume',v]);}
  setMuted(v){this.muted=v;window.__sdkCalls.push(['muted',v]);}
  getVolume(){return this.volume;}
  getMuted(){return this.muted;}
  play(){window.__sdkCalls.push(['play']);this.fire('blocked');}
}
Object.assign(FakePlayer,{READY:'ready',PLAY:'play',PLAYING:'playing',PAUSE:'pause',PLAYBACK_BLOCKED:'blocked',OFFLINE:'offline',ENDED:'ended'});
window.Twitch={Player:FakePlayer};
'''

def document(page, folder, fragment=""):
    # An in-memory DOM harness avoids all network access. Production files are unchanged.
    # CSP and module loading require native/integration tests, not this mocked harness.
    page.goto("about:blank" + ("#" + fragment if fragment else ""))
    html=(ROOT/folder/'index.html').read_text()
    html=re.sub(r'<script\b[^>]*>.*?</script>', '', html, flags=re.S)
    html=re.sub(r'<link\b[^>]*>', '', html)
    html=re.sub(r'<meta http-equiv="Content-Security-Policy"[^>]*>', '', html)
    page.set_content(html)
    page.add_style_tag(content=(ROOT/folder/'style.css').read_text())

results=[]
def check(name,condition):
    assert condition,name
    results.append(name)
    print('PASS',name)
try:
    with sync_playwright() as p:
        launch={'headless':True,'args':['--no-sandbox']}
        if os.environ.get('MPD_TEST_CHROMIUM'):launch['executable_path']=os.environ['MPD_TEST_CHROMIUM']
        browser=p.chromium.launch(**launch)
        page=browser.new_page(viewport={'width':1280,'height':1100},device_scale_factor=1)
        errors=[]
        page.on('pageerror',lambda e:errors.append(str(e)))
        document(page,'ui')
        page.evaluate((ROOT/'tests/ui-fixture.js').read_text())
        helpers=(ROOT/'ui/view-model.js').read_text().replace('export function ', 'function ')
        app=re.sub(r'^import[^\n]*\n', '', (ROOT/'ui/app.js').read_text(), count=1)
        page.evaluate('() => {'+helpers+'\n'+app+'\n}')
        page.wait_for_selector('.favorite')
        check('Manager renders four ranked favorites',page.locator('.favorite').count()==4)
        check('Public Client ID has no editable field',page.locator('#client-id').count()==0)
        check('Manager renders three player cards',page.locator('.player-card').count()==3)
        check('Simulated data is visibly distinguished',page.locator('#playing-heading').inner_text()=='SIMULATED PLAYBACK')
        check('First channel cannot be moved above rank one',page.get_by_role('button',name='Move alpha_demo up',exact=True).is_disabled())
        (ROOT/'docs').mkdir(exist_ok=True)
        page.screenshot(path=str(ROOT/'docs/ui-preview.png'),full_page=True)
        page.get_by_role('button',name='Move delta_demo up',exact=True).click()
        page.wait_for_function('window.__actions.some(a=>a.type==="move")')
        check('Reorder dispatches the correct zero-based target',page.evaluate('window.__actions.find(a=>a.type==="move")')=={'type':'move','login':'delta_demo','position':2})
        page.locator('#volume').fill('40')
        page.wait_for_function('window.__actions.some(a=>a.type==="set_audio"&&a.volume===40)')
        check('Volume dispatch preserves independent mute',page.evaluate('window.__actions.filter(a=>a.type==="set_audio").at(-1)')=={'type':'set_audio','volume':40,'muted':False})
        page.locator('#channel').fill('new_channel')
        page.get_by_role('button',name='Add channel',exact=True).click()
        page.wait_for_selector('.favorite[data-login="new_channel"]')
        check('Add-channel form sends a manager action',page.locator('.favorite').count()==5)
        page.locator('#channel').fill('bad!input')
        page.get_by_role('button',name='Add channel',exact=True).click()
        page.wait_for_selector('#error:not([hidden])')
        check('Rejected input is preserved for correction',page.locator('#channel').input_value()=='bad!input')
        page.locator('#pause').click()
        page.wait_for_function('document.querySelector("#start").textContent==="Resume"')
        check('Pause retains displayed player sessions',page.locator('.player-card').count()==3)
        page.locator('#stop').click()
        page.wait_for_function('document.querySelectorAll(".player-card").length===0')
        check('Stop action updates empty-session UI',page.locator('#empty-players').is_visible())
        check('Demo cannot start Twitch connection',page.locator('#connect').is_disabled())
        page.evaluate('window.__fixture.settings.demo=false')
        page.wait_for_function('!document.getElementById("connect").disabled')
        page.locator('#connect').click()
        page.wait_for_function('window.__actions.some(a=>a.type==="connect")')
        check('Connect has no client ID override',page.evaluate('window.__actions.filter(a=>a.type==="connect").at(-1)')=={'type':'connect'})
        check('Manager ran without JavaScript exceptions',not errors)

        wrapper=browser.new_page(viewport={'width':820,'height':550})
        wrapper_errors=[]
        wrapper.on('pageerror',lambda e:wrapper_errors.append(str(e)))
        document(wrapper,'player-wrapper','channel=modpackdad&session=42&volume=25&muted=false&demo=false')
        wrapper.evaluate('window.__reports=[];window.__TAURI__={core:{invoke:async(c,a)=>{window.__reports.push({command:c,...a});}}};')
        wrapper.evaluate(FAKE_SDK)
        wrapper.evaluate("""() => { const append=document.head.append.bind(document.head);
          document.head.append=(...nodes)=>{for(const n of nodes){
            if(n.tagName==='SCRIPT'&&n.src==='https://player.twitch.tv/js/embed/v1.js'){
              window.__requestedSdk=n.src;queueMicrotask(()=>n.onload());
            }else append(n);
          }}; }""")
        wrapper.evaluate((ROOT/'player-wrapper/player.js').read_text())
        wrapper.wait_for_function('window.__sdk!==undefined')
        check('Wrapper derives parent from its document hostname',wrapper.evaluate('window.__sdkOptions.parent[0]===location.hostname'))
        check('SDK initializes without uncontrolled autoplay/audio',wrapper.evaluate('[window.__sdkOptions.autoplay,window.__sdkOptions.muted]')==[False,True])
        wrapper.evaluate('window.__sdk.fire("ready")')
        wrapper.wait_for_function('window.__reports.some(x=>x.report.state==="blocked")')
        calls=wrapper.evaluate('window.__sdkCalls')
        check('Requested audio is applied before the first play attempt',calls[:3]==[['volume',.25],['muted',False],['play']])
        check('Blocked autoplay is reported rather than called playing',wrapper.evaluate('window.__reports.at(-1).report.state')=='blocked')
        wrapper.evaluate('window.__sdk.fire("pause"); window.mpdSetAudio(0,false)')
        wrapper.wait_for_timeout(5200)
        check('Heartbeat does not unpause or repeatedly call play',wrapper.evaluate('window.__sdkCalls.filter(x=>x[0]==="play").length')==1)
        check('Zero volume and unmuted remain distinct',wrapper.evaluate('[window.__sdk.volume,window.__sdk.muted]')==[0,False])
        check('Player reports use only the narrow native command',wrapper.evaluate('window.__reports.every(x=>x.command==="player_report"&&x.report.session===42)'))
        check('Wrapper ran without JavaScript exceptions',not wrapper_errors)

        demo=browser.new_page(viewport={'width':820,'height':550})
        requests=[]
        demo.on('request',lambda req:requests.append(req.url))
        document(demo,'player-wrapper','channel=alpha_demo&session=9&volume=25&muted=false&demo=true')
        demo.evaluate('window.__reports=[];window.__TAURI__={core:{invoke:async(c,a)=>window.__reports.push(a)}};')
        demo.evaluate((ROOT/'player-wrapper/player.js').read_text())
        demo.wait_for_function('window.__reports.length>0')
        check('Demo does not load Twitch or stream media',not any('twitch.tv' in r for r in requests))
        check('Demo visibly states that no viewers are created','No Twitch video or viewers' in demo.locator('#demo').inner_text())
        demo.get_by_role('button',name='Autoplay blocked',exact=True).click()
        demo.wait_for_function('window.__reports.at(-1).report.state==="blocked"')
        check('Demo lets the user exercise blocked telemetry',demo.locator('#status').inner_text()=='Simulated · blocked')
        demo.screenshot(path=str(ROOT/'docs/player-preview.png'),full_page=True)
        browser.close()
finally:
    pass
(ROOT/'docs/browser-test-results.json').write_text(json.dumps({'kind':'mocked browser UI and SDK tests; NOT native or live Twitch tests','passed':len(results),'tests':results},indent=2)+'\n')
print(f'{len(results)} browser checks passed.')
