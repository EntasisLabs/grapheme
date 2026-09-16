import { error } from '@sveltejs/kit';
import { DOC_NAV, titleFromSlug } from '$lib/docs';
import { renderMarkdown } from '$lib/markdown';
import type { EntryGenerator, PageServerLoad } from './$types';

export const prerender = true;

export const entries: EntryGenerator = () => DOC_NAV.map((d) => ({ slug: d.slug }));

const modules = import.meta.glob('../../../../content/docs/**/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

function resolveDoc(slug: string): string | undefined {
	const normalized = Object.entries(modules).map(([k, v]) => [k.replaceAll('\\', '/'), v] as const);
	const needles = slug === 'tutorials' ? ['/tutorials/README.md'] : [`/docs/${slug}.md`, `/${slug}.md`];
	for (const needle of needles) {
		const hit = normalized.find(([k]) => k.endsWith(needle));
		if (hit) return hit[1];
	}
	return undefined;
}

/** First prose paragraph of a markdown doc, stripped of inline markup, for meta descriptions. */
function firstParagraph(md: string): string {
	const para = md
		.split(/\n\s*\n/)
		.map((p) => p.trim())
		.find((p) => p && !/^(#|```|[-*]\s|\d+\.\s|\||>|<)/.test(p));
	if (!para) return 'Grapheme documentation.';
	return para
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/[`*_]/g, '')
		.replace(/\s+/g, ' ')
		.slice(0, 200);
}

export const load: PageServerLoad = async ({ params }) => {
	const slug = params.slug;
	const source = resolveDoc(slug);
	if (!source) error(404, `Doc not found: ${slug}`);

	const idx = DOC_NAV.findIndex((d) => d.slug === slug);
	const prev = idx > 0 ? DOC_NAV[idx - 1] : null;
	const next = idx >= 0 && idx < DOC_NAV.length - 1 ? DOC_NAV[idx + 1] : null;

	const html = await renderMarkdown(source);
	return {
		slug,
		title: titleFromSlug(slug),
		description: firstParagraph(source),
		section: DOC_NAV[idx]?.section ?? 'Docs',
		html,
		prev,
		next,
		editUrl: `https://github.com/EntasisLabs/grapheme/edit/main/docs/${slug === 'tutorials' ? 'tutorials/README' : slug}.md`
	};
};
