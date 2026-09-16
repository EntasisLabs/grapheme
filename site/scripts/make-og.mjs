// Renders scripts/og.html to static/og.png (1200x630) with headless Chrome via CDP.
// Usage: node scripts/make-og.mjs   (requires google-chrome / chromium on PATH)
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const html = `file://${join(root, 'scripts', 'og.html')}`;
const out = join(root, 'static', 'og.png');

const chrome = ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']
	.map((bin) => {
		try {
			return execFileSync('which', [bin]).toString().trim();
		} catch {
			return '';
		}
	})
	.find(Boolean);

if (!chrome) {
	console.error('make-og: no Chrome/Chromium binary found; skipping OG render.');
	process.exit(existsSync(out) ? 0 : 1);
}

const profile = mkdtempSync(join(tmpdir(), 'og-'));
const proc = spawn(
	chrome,
	[
		'--headless=new',
		'--no-sandbox',
		'--disable-gpu',
		'--hide-scrollbars',
		'--force-device-scale-factor=1',
		`--user-data-dir=${profile}`,
		'--remote-debugging-port=0',
		'--window-size=1200,630',
		'about:blank'
	],
	{ stdio: 'ignore' }
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const portFile = join(profile, 'DevToolsActivePort');
let port = 0;
for (let i = 0; i < 100 && !port; i++) {
	await sleep(100);
	if (existsSync(portFile)) port = Number(readFileSync(portFile, 'utf8').split('\n')[0]);
}
if (!port) {
	proc.kill();
	throw new Error('make-og: Chrome did not expose a DevTools port');
}

const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const target = targets.find((t) => t.type === 'page');
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (ev) => {
	const m = JSON.parse(ev.data);
	if (m.id && pending.has(m.id)) {
		pending.get(m.id)(m);
		pending.delete(m.id);
	}
};
const send = (method, params = {}) =>
	new Promise((res) => {
		const i = ++id;
		pending.set(i, res);
		ws.send(JSON.stringify({ id: i, method, params }));
	});

await send('Page.enable');
await send('Emulation.setDeviceMetricsOverride', { width: 1200, height: 630, deviceScaleFactor: 1, mobile: false });
await send('Page.navigate', { url: `${html}?v=${Date.now()}` });
// wait for load + web fonts (Syne and Plex Mono are fetched from Google Fonts)
const fontsReady = async () =>
	(
		await send('Runtime.evaluate', {
			expression: `document.readyState === 'complete' && document.fonts.check('600 20px "Source Serif 4"') && document.fonts.check('700 16px "JetBrains Mono"')`,
			returnByValue: true
		})
	).result?.result?.value === true;
for (let i = 0; i < 80 && !(await fontsReady()); i++) await sleep(100);
if (!(await fontsReady())) console.warn('make-og: web fonts did not load; rendering with fallbacks');
await sleep(300);
const shot = await send('Page.captureScreenshot', {
	format: 'png',
	clip: { x: 0, y: 0, width: 1200, height: 630, scale: 1 }
});
writeFileSync(out, Buffer.from(shot.result.data, 'base64'));
ws.close();
proc.kill();
await new Promise((r) => proc.once('exit', r));
try {
	rmSync(profile, { recursive: true, force: true });
} catch {
	// temp profile cleanup is best-effort
}
console.log(`make-og: wrote ${out}`);
