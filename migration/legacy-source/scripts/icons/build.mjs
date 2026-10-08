#!/usr/bin/env node
// Regenerates every platform's app icon from branding/icon.svg and branding/icon-fullbleed.svg (see branding/README.md).
// Needs Playwright with Chromium (set PLAYWRIGHT_MODULE to its path if it isn't resolvable) and, for the launcher, its npm install.
import { readFileSync, writeFileSync, mkdtempSync, copyFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

const icon = readFileSync(join(root, 'branding/icon.svg'), 'utf8');
const bleed = readFileSync(join(root, 'branding/icon-fullbleed.svg'), 'utf8');
const BG = '#16181e';

const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, args: ['--no-sandbox'] });
const page = await browser.newPage();

/** Render an SVG (or HTML body) to a PNG of `size` x `size` (or w x h), keeping transparency when asked. */
async function png(svg, w, h = w, { transparent = true, background = BG, scale = 1 } = {}) {
  await page.setViewportSize({ width: w, height: h });
  await page.setContent(`<style>html,body{margin:0;background:${transparent ? 'transparent' : background}}svg{display:block;width:${w}px;height:${h}px}</style>${svg}`);
  return page.screenshot({ type: 'png', omitBackground: transparent, clip: { x: 0, y: 0, width: w, height: h } });
}
const out = (rel, data) => { const p = join(root, rel); mkdirSync(dirname(p), { recursive: true }); writeFileSync(p, data); console.log('wrote', rel); };

// Website + installable web app.
out('panel/web/public/favicon.svg', icon);
out('panel/web/public/icon-192.png', await png(icon, 192));
out('panel/web/public/icon-512.png', await png(icon, 512));
out('panel/web/public/icon-maskable-512.png', await png(bleed, 512, 512, { transparent: false }));
out('panel/web/public/apple-touch-icon.png', await png(bleed, 180, 180, { transparent: false }));

// Fabric companion mod.
out('integrations/fabric-client/src/26/resources/assets/scopenet_client/icon.png', await png(icon, 128));

// Android / iOS: full bleed icon and a splash with the mark centred.
out('mobile/assets/icon.png', await png(bleed, 1024, 1024, { transparent: false }));
const splash = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2732 2732"><rect width="2732" height="2732" fill="${BG}"/><g transform="translate(1366 1366) scale(0.55) translate(-512 -512)">${bleed.replace(/^[\s\S]*?<g[^>]*>/, '').replace(/<\/g><\/svg>\s*$/, '')}</g></svg>`;
out('mobile/assets/splash.png', await png(splash, 2732, 2732, { transparent: false }));

// Desktop launcher: Tauri turns one 1024px PNG into every size, .ico and .icns.
const tmp = mkdtempSync(join(tmpdir(), 'scopenet-icons-'));
writeFileSync(join(tmp, 'source.png'), await png(icon, 1024));
const tauri = join(root, 'launcher/node_modules/.bin/tauri');
execFileSync(tauri, ['icon', join(tmp, 'source.png'), '-o', join(tmp, 'out')], { stdio: 'inherit', cwd: join(root, 'launcher') });
for (const f of ['32x32.png', '64x64.png', '128x128.png', '128x128@2x.png', 'icon.png', 'icon.ico', 'icon.icns']) copyFileSync(join(tmp, 'out', f), join(root, 'launcher/src-tauri/icons', f));
copyFileSync(join(root, 'branding/icon.svg'), join(root, 'launcher/src-tauri/icons/logo.svg'));
console.log('wrote launcher/src-tauri/icons/*');

await browser.close();
