import { marked } from 'marked';
import { codeToHtml } from 'shiki';

marked.setOptions({
	gfm: true,
	breaks: false
});

export async function renderMarkdown(source: string): Promise<string> {
	const rewritten = rewriteDocLinks(source);
	const html = await marked.parse(rewritten);
	return enhanceCodeBlocks(html);
}

/** Point repo-relative links at GitHub so static hosting does not 404. */
function rewriteDocLinks(source: string): string {
	return source
		.replace(/\]\(\.\.\/CHANGELOG\.md(#[^)]*)?\)/g, '](https://github.com/EntasisLabs/grapheme/blob/main/CHANGELOG.md$1)')
		.replace(/\]\(\.\.\/examples\//g, '](https://github.com/EntasisLabs/grapheme/tree/main/examples/')
		.replace(/\]\(examples\//g, '](https://github.com/EntasisLabs/grapheme/tree/main/examples/')
		.replace(/\]\(docs\/internal\//g, '](https://github.com/EntasisLabs/grapheme/tree/main/docs/internal/')
		.replace(/\]\(\.\.\/docs\/internal\//g, '](https://github.com/EntasisLabs/grapheme/tree/main/docs/internal/')
		.replace(/\]\(docs\/([^)#]+)\.md(#[^)]*)?\)/g, '](/docs/$1$2)')
		.replace(/\]\(\.\/([^)#]+)\.md(#[^)]*)?\)/g, '](/docs/$1$2)')
		.replace(/\]\(([^)/#:]+)\.md(#[^)]*)?\)/g, '](/docs/$1$2)');
}

async function enhanceCodeBlocks(html: string): Promise<string> {
	const re = /<pre><code class="language-([^"]+)">([\s\S]*?)<\/code><\/pre>/g;
	const matches = [...html.matchAll(re)];
	if (matches.length === 0) return html;

	let out = html;
	for (const match of matches) {
		const [full, lang, encoded] = match;
		const code = decodeHtml(encoded ?? '');
		const mapped = mapLang(lang ?? '');
		try {
			const highlighted = await codeToHtml(code, {
				lang: mapped,
				theme: 'everforest-light'
			});
			out = out.replace(full, highlighted);
		} catch {
			// keep original block
		}
	}
	return out;
}

function mapLang(lang: string): string {
	const l = lang.toLowerCase();
	if (l === 'gr' || l === 'grapheme') return 'rust';
	if (l === 'sh' || l === 'shell' || l === 'zsh') return 'bash';
	if (l === 'yml') return 'yaml';
	return l || 'text';
}

function decodeHtml(s: string): string {
	return s
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&amp;/g, '&')
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'");
}
