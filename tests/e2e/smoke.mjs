// Drives the built app in Chromium: opens the example, edits, runs checks
// and exports. Fails on any console error.
//
//   cd app && trunk build && cd ../tests/e2e && npm i && node smoke.mjs [url]
//
// Set CHROMIUM_PATH to use a preinstalled browser.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const dist = fileURLToPath(new URL('../../app/dist/', import.meta.url));
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' };

function serve() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^([/\\])+/, '');
    try {
      const body = await readFile(join(dist, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path)] || 'text/html' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  return new Promise((resolve) => server.listen(0, () => resolve(server)));
}

function check(cond, msg) {
  if (!cond) throw new Error('FAILED: ' + msg);
  console.log('ok -', msg);
}

const server = await serve();
const url = process.argv[2] || `http://localhost:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH || undefined,
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'],
});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(e.message));
page.on('dialog', (d) => d.accept(d.defaultValue() || 'NET1'));

try {
  await page.goto(url, { waitUntil: 'networkidle' });
  await page.waitForSelector('.topbar');
  check(await page.isVisible('text=Quick Start'), 'start page renders');

  await page.click('.tile:has-text("Open Example")');
  await page.waitForSelector('.editor canvas');
  check(await page.isVisible('.tab.active:has-text("Sheet1")'), 'example opens on the schematic');

  // Draw a wire, then undo it.
  const canvas = page.locator('.editor canvas');
  await canvas.click({ position: { x: 700, y: 150 } });
  await page.keyboard.press('w');
  await canvas.click({ position: { x: 700, y: 150 } });
  await canvas.click({ position: { x: 800, y: 200 } });
  await canvas.dblclick({ position: { x: 800, y: 200 } });
  await page.keyboard.press('Escape');
  check((await page.textContent('.statusbar')).includes('Wire'), 'wire tool adds a wire');
  await page.keyboard.press('Control+z');
  check((await page.textContent('.bottom')).includes('Undo: Draw wire'), 'undo reverts it');

  // ERC from the toolbar.
  await page.click('.toolbar button[aria-label="Electrical Rule Check"]');
  check((await page.textContent('.bottom-tabs')).includes('ERC'), 'ERC runs');

  // Palette jumps to a part.
  await page.keyboard.press('Control+k');
  await page.keyboard.type('U1');
  await page.keyboard.press('Enter');
  check((await page.textContent('.props')).includes('NE555'), 'palette selects U1 and shows its properties');

  // PCB, DRC and fabrication files.
  await page.click('.tab:has-text("PCB")');
  await page.waitForTimeout(500);
  await page.keyboard.press('Control+Alt+d');
  check((await page.textContent('.bottom-tabs')).includes('DRC'), 'DRC runs');
  await page.keyboard.press('Control+Alt+g');
  await page.waitForSelector('.modal:has-text("Fabrication Package")');
  const files = await page.textContent('.fab-right');
  check(files.includes('.GTL') && files.includes('.DRL') && files.includes('gbrjob'), 'fab package lists Gerber, drill and job files');
  await page.keyboard.press('Escape');

  // 3D view builds.
  await page.keyboard.press('Alt+3');
  await page.waitForSelector('.view3d canvas');
  check(await page.isVisible('.view3d-bar'), '3D view opens');

  check(errors.length === 0, 'no console errors' + (errors.length ? ': ' + errors.join(' | ') : ''));
} finally {
  await browser.close();
  server.close();
}
