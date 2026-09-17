import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const site = join(root, 'site');
const candidates = [
	join(root, 'target/wasm32-wasip1/release/grapheme-wasm.wasm'),
	process.env.CARGO_TARGET_DIR
		? join(process.env.CARGO_TARGET_DIR, 'wasm32-wasip1/release/grapheme-wasm.wasm')
		: null
].filter(Boolean);

const src = candidates.find((p) => existsSync(p));
if (!src) {
	console.warn(
		'[sync-wasm] grapheme-wasm.wasm not found. Run: bash scripts/build-runtime-wasm.sh'
	);
	process.exit(0);
}

const destDir = join(site, 'static');
mkdirSync(destDir, { recursive: true });
const dest = join(destDir, 'grapheme-wasm.wasm');
copyFileSync(src, dest);
console.log(`[sync-wasm] ${src} -> ${dest}`);
