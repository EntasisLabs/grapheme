import { cpSync, existsSync, mkdirSync, readdirSync, rmSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const docsSrc = join(root, 'docs');
const dest = join(root, 'site/content/docs');

if (!existsSync(docsSrc)) {
	console.warn('[sync-docs] docs/ not found');
	process.exit(0);
}

rmSync(dest, { recursive: true, force: true });
mkdirSync(dest, { recursive: true });

function copyMd(from, to) {
	mkdirSync(to, { recursive: true });
	for (const entry of readdirSync(from)) {
		if (entry === 'internal') continue;
		const srcPath = join(from, entry);
		const destPath = join(to, entry);
		const st = statSync(srcPath);
		if (st.isDirectory()) {
			copyMd(srcPath, destPath);
		} else if (entry.endsWith('.md')) {
			cpSync(srcPath, destPath);
		}
	}
}

copyMd(docsSrc, dest);
console.log(`[sync-docs] mirrored product docs -> ${dest}`);
