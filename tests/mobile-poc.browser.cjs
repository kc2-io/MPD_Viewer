const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { chromium } = require('playwright');

const root = path.join(__dirname, '..', 'mobile-poc', 'ui');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' };

function server() {
  return http.createServer((request, response) => {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    const relative = pathname === '/' ? 'index.html' : pathname.slice(1);
    const filename = path.resolve(root, relative);
    if (!filename.startsWith(`${root}${path.sep}`) || !fs.existsSync(filename)) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { 'Content-Type': types[path.extname(filename)] || 'application/octet-stream' });
    fs.createReadStream(filename).pipe(response);
  });
}

function chromiumExecutable() {
  if (process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE) return process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE;
  const cache = path.join(os.homedir(), '.cache', 'ms-playwright');
  if (!fs.existsSync(cache)) return undefined;
  for (const directory of fs.readdirSync(cache).sort().reverse()) {
    for (const relative of ['chrome-linux64/chrome', 'chrome-linux/chrome', 'chrome-linux/headless_shell']) {
      const candidate = path.join(cache, directory, relative);
      if (fs.existsSync(candidate)) return candidate;
    }
  }
  return undefined;
}

(async () => {
  const host = server();
  await new Promise(resolve => host.listen(0, '127.0.0.1', resolve));
  const port = host.address().port;
  const browser = await chromium.launch({ executablePath: chromiumExecutable(), headless: true });
  try {
    for (const viewport of [{ width: 390, height: 844 }, { width: 412, height: 915 }, { width: 820, height: 1180 }]) {
      const page = await browser.newPage({ viewport });
      const external = [];
      page.on('request', request => {
        if (!request.url().startsWith(`http://127.0.0.1:${port}/`)) external.push(request.url());
      });
      await page.goto(`http://127.0.0.1:${port}/?preview=1`);
      await page.getByRole('button', { name: 'Start' }).click();
      await page.locator('.stream-tile').first().waitFor();
      assert.equal(await page.locator('.stream-tile').count(), 4, `four tiles at ${viewport.width}px`);
      assert.equal(await page.locator('#session-count').textContent(), '4 / 4');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `no horizontal overflow at ${viewport.width}px`);
      if (process.env.MOBILE_POC_SCREENSHOT && viewport.width === 390) {
        await page.screenshot({ path: process.env.MOBILE_POC_SCREENSHOT, fullPage: true });
      }
      await page.getByRole('button', { name: 'Audio + chat' }).click();
      assert.equal(await page.locator('.audio-card').count(), 4, `four audio sessions at ${viewport.width}px`);
      assert.equal(await page.locator('#audio-view').isVisible(), true);
      assert.equal(await page.locator('.chat-panel').isVisible(), true);
      assert.deepEqual(external, [], 'preview must not request Twitch or other external media');
      await page.close();
    }
    console.log('mobile POC browser checks passed: phone, large phone, tablet');
  } finally {
    await browser.close();
    await new Promise(resolve => host.close(resolve));
  }
})().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
