"""Browser checks at the configured HTTPS origin, served entirely by route mocks.

Unmodified wrapper HTML/CSS/JS and its CSP are used. HTTPS/TLS, the deployed
website, the actual Twitch SDK/media, and native Tauri IPC are NOT exercised.
Run with Python + Playwright. MPD_TEST_CHROMIUM selects a system Chromium.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
from urllib.parse import urljoin, urlsplit

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
BASE = json.loads((ROOT / 'src-tauri/player-origin.json').read_text())['url']
SDK_URL = 'https://player.twitch.tv/js/embed/v1.js'
REPORT = ROOT / 'docs/hosted-wrapper-test-results.json'
REPORT.unlink(missing_ok=True)
if not BASE:
    raise SystemExit('Configure an HTTPS player URL before running this hosted-origin test.')

FAKE_SDK = r'''
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

results: list[str] = []
def check(name: str, condition: bool) -> None:
    if not condition:
        raise AssertionError(name)
    results.append(name)
    print('PASS', name)

assets = {
    BASE: ('text/html', (ROOT / 'player-wrapper/index.html').read_text()),
    urljoin(BASE, 'style.css'): ('text/css', (ROOT / 'player-wrapper/style.css').read_text()),
    urljoin(BASE, 'player.js'): ('application/javascript', (ROOT / 'player-wrapper/player.js').read_text()),
    SDK_URL: ('application/javascript', FAKE_SDK),
}
requests: list[str] = []
unexpected: list[str] = []
errors: list[str] = []
with sync_playwright() as p:
    launch = {'headless': True, 'args': ['--no-sandbox']}
    if os.environ.get('MPD_TEST_CHROMIUM'):
        launch['executable_path'] = os.environ['MPD_TEST_CHROMIUM']
    browser = p.chromium.launch(**launch)
    context = browser.new_context(viewport={'width': 820, 'height': 550}, service_workers='block')
    context.add_init_script('''window.__reports=[];window.__cspViolations=[];
        document.addEventListener('securitypolicyviolation',e=>window.__cspViolations.push(e.violatedDirective));
        window.__TAURI__={core:{invoke:async(command,args)=>{window.__reports.push({command,...args});}}};''')

    def route_handler(route):
        request_url = route.request.url.split('#', 1)[0]
        requests.append(request_url)
        if request_url in assets:
            content_type, body = assets[request_url]
            route.fulfill(status=200, content_type=content_type, body=body,
                headers={'X-Content-Type-Options': 'nosniff',
                         'Referrer-Policy': 'strict-origin-when-cross-origin'})
        else:
            unexpected.append(request_url)
            route.abort()

    context.route('**/*', route_handler)
    page = context.new_page()
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE + '#channel=modpackdad&session=42&volume=25&muted=false&demo=false')
    page.wait_for_function('window.__sdk !== undefined')
    check('Unmodified hosted wrapper assets load with their CSP retained',
        all(url in requests for url in assets)
        and page.locator('meta[http-equiv="Content-Security-Policy"]').count() == 1
        and not page.evaluate('window.__cspViolations'))
    check('Twitch parent equals the actual configured document hostname',
        page.evaluate('window.__sdkOptions.parent') == [urlsplit(BASE).hostname]
        and page.evaluate('location.hostname') == urlsplit(BASE).hostname)
    check('Fragment supplies the channel without query parameters in asset requests',
        page.evaluate('window.__sdkOptions.channel') == 'modpackdad'
        and all(not urlsplit(url).query for url in requests))
    page.evaluate('window.__sdk.fire("ready")')
    page.wait_for_function('window.__reports.some(x=>x.report.state==="blocked")')
    check('Hosted wrapper applies requested audio before its first play attempt',
        page.evaluate('window.__sdkCalls.slice(0,3)') == [['volume', .25], ['muted', False], ['play']])
    check('Hosted wrapper displays blocked autoplay rather than claiming playback',
        page.locator('#status').inner_text() == 'blocked'
        and 'Autoplay was blocked' in page.locator('#message').inner_text())
    check('Hosted wrapper sends only advisory reports for its own session',
        page.evaluate('window.__reports.length>0 && window.__reports.every(x=>x.command==="player_report"&&x.report.session===42)'))
    page.evaluate('window.__sdk.fire("pause");window.mpdSetAudio(0,true)')
    check('Changing hosted audio does not start a paused player',
        page.evaluate('[window.__sdk.volume,window.__sdk.muted,window.__sdkCalls.filter(x=>x[0]==="play").length]') == [0, True, 1])
    check('Hosted-origin mock has no script errors or unexpected network requests',
        not errors and not unexpected and not page.evaluate('window.__cspViolations'))
    browser.close()

REPORT.write_text(json.dumps({
    'kind': 'HTTPS-origin ROUTE MOCK; unchanged assets/CSP; mock Twitch SDK and mock IPC; NOT deployed-site, TLS, native or media validation',
    'configured_url': BASE,
    'passed': len(results),
    'tests': results,
}, indent=2) + '\n')
print(f'{len(results)} hosted-origin mock checks passed.')
