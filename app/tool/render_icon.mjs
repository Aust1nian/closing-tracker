// Draft app icon: the Tight Line mark (the plumb bob) on white, 1024 px.
// Austin approves the icon before store submission (spec open question 4).
//
// usage: node tool/render_icon.mjs <Tight_Line_Website repo> <out.png>
// Uses the website scripts/ playwright-core and Playwright's cached headless Chromium
// (set CHROMIUM to use another). The PNG still has an alpha channel, which
// Apple refuses, so tool/flatten_icon.py writes the final opaque files.
import fs from 'node:fs';
import { createRequire } from 'node:module';

const [site, out] = process.argv.slice(2);
if (!site || !out) {
  console.error('usage: node tool/render_icon.mjs <Tight_Line_Website repo> <out.png>');
  process.exit(2);
}
const { chromium } = createRequire(`${site}/scripts/`)('playwright-core');
const svg = fs.readFileSync(`${site}/site/public/brand/mark.svg`).toString('base64');
const exe =
  process.env.CHROMIUM ??
  `${process.env.HOME}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`;

const SIZE = 1024;
const MARK = Math.round(SIZE * 0.62);
const browser = await chromium.launch({ executablePath: exe });
const page = await browser.newPage({ viewport: { width: SIZE, height: SIZE }, colorScheme: 'light' });
await page.setContent(
  `<body style="margin:0;width:${SIZE}px;height:${SIZE}px;display:grid;place-items:center;background:#FFFFFF">` +
    `<img src="data:image/svg+xml;base64,${svg}" style="width:${MARK}px;height:${MARK}px"></body>`,
);
await page.waitForLoadState('load');
fs.writeFileSync(out, await page.screenshot());
await browser.close();
console.log(`wrote ${out}`);
