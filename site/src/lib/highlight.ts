/**
 * Hand-rolled Grapheme (.gr) tokenizer → highlighted HTML.
 * Derived from crates/grapheme-compiler/src/grapheme.pest.
 */

const KEYWORDS = new Set([
	'import', 'from', 'types',
	'glyph', 'query', 'mutation', 'iterator', 'node', 'fragment', 'subscription',
	'struct', 'enum', 'state_machine', 'transition', 'terminal', 'type', 'schema', 'propose', 'module',
	'tag', 'for', 'using', 'const', 'mutable',
	'call', 'set', 'apply', 'match', 'case', 'default', 'if', 'then', 'else', 'return',
	'on', 'state'
]);

const LITERALS = new Set(['true', 'false', 'null']);
const SCALARS = new Set(['String', 'Int', 'Float', 'Bool', 'Any', 'Json']);

type Tok = { cls: string | null; text: string };

function esc(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export function tokenize(src: string): Tok[] {
	const out: Tok[] = [];
	let i = 0;
	const n = src.length;

	const push = (cls: string | null, text: string) => {
		if (text) out.push({ cls, text });
	};

	while (i < n) {
		const c = src[i]!;

		// line comment
		if (c === '/' && src[i + 1] === '/') {
			let j = i;
			while (j < n && src[j] !== '\n') j++;
			push('cm', src.slice(i, j));
			i = j;
			continue;
		}

		// string (with {interpolation} highlighted)
		if (c === '"') {
			let j = i + 1;
			while (j < n && src[j] !== '"') {
				if (src[j] === '\\') j++;
				j++;
			}
			const raw = src.slice(i, Math.min(j + 1, n));
			// split interpolation segments
			const parts = raw.split(/(\{\$[a-zA-Z_][\w.]*\})/g);
			for (const p of parts) {
				if (/^\{\$/.test(p)) push('interp', p);
				else push('str', p);
			}
			i = j + 1;
			continue;
		}

		// variable $state.x
		if (c === '$') {
			let j = i + 1;
			while (j < n && /[\w.]/.test(src[j]!)) j++;
			push('var', src.slice(i, j));
			i = j;
			continue;
		}

		// directive @loop
		if (c === '@') {
			let j = i + 1;
			while (j < n && /\w/.test(src[j]!)) j++;
			push('dir', src.slice(i, j));
			i = j;
			continue;
		}

		// pipe / arrows
		if (c === '|' && src[i + 1] === '>') {
			push('pipe', '|>');
			i += 2;
			continue;
		}
		if ((c === '-' || c === '=') && src[i + 1] === '>') {
			push('arrow', src.slice(i, i + 2));
			i += 2;
			continue;
		}
		if (/[=<>!]/.test(c) && src[i + 1] === '=') {
			push('op', src.slice(i, i + 2));
			i += 2;
			continue;
		}
		if (/[<>]/.test(c)) {
			push('op', c);
			i++;
			continue;
		}

		// number
		if (/\d/.test(c) || (c === '-' && /\d/.test(src[i + 1] ?? ''))) {
			let j = i + 1;
			while (j < n && /[\d.]/.test(src[j]!)) j++;
			push('num', src.slice(i, j));
			i = j;
			continue;
		}

		// identifier / keyword / type / module.op
		if (/[A-Za-z_]/.test(c)) {
			let j = i;
			while (j < n && /\w/.test(src[j]!)) j++;
			const word = src.slice(i, j);

			// module.op  (lowercase ident followed by .ident and '(' or whitespace)
			if (src[j] === '.' && /[a-z_]/.test(src[j + 1] ?? '') && /^[a-z_]\w*$/.test(word)) {
				let k = j + 1;
				while (k < n && /\w/.test(src[k]!)) k++;
				push('mod', word);
				push(null, '.');
				push('fn', src.slice(j + 1, k));
				i = k;
				continue;
			}

			if (KEYWORDS.has(word)) push('kw', word);
			else if (LITERALS.has(word)) push('lit', word);
			else if (SCALARS.has(word)) push('type', word);
			else if (/^[A-Z]/.test(word)) push('def', word);
			else if (src[j] === '(' ) push('fn', word);
			else if (src[j] === ':' ) push('key', word);
			else push(null, word);
			i = j;
			continue;
		}

		// punctuation & whitespace
		if (/[{}()\[\],:.]/.test(c)) {
			push('p', c);
			i++;
			continue;
		}

		push(null, c);
		i++;
	}

	return out;
}

export function highlightGr(src: string): string {
	return tokenize(src)
		.map((t) => (t.cls ? `<span class="t-${t.cls}">${esc(t.text)}</span>` : esc(t.text)))
		.join('');
}

/** Minimal JSON highlighter for result panes. */
export function highlightJson(text: string): string {
	return esc(text).replace(
		/("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false|null)\b|(-?\d+(?:\.\d+)?(?:e[+-]?\d+)?)/gi,
		(m, str, colon, lit, num) => {
			if (str) return colon ? `<span class="t-key">${str}</span>${colon}` : `<span class="t-str">${str}</span>`;
			if (lit) return `<span class="t-lit">${lit}</span>`;
			if (num) return `<span class="t-num">${num}</span>`;
			return m;
		}
	);
}
