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
	const needles =
		slug === 'tutorials'
			? ['/tutorials/README.md']
			: [`/docs/${slug}.md`, `/${slug}.md`];

	for (const needle of needles) {
		const hit = normalized.find(([k]) => k.endsWith(needle));
		if (hit) return hit[1];
	}
	return undefined;
}

export const load: PageServerLoad = async ({ params }) => {
	const slug = params.slug;
	const source = resolveDoc(slug);
	if (!source) {
		error(404, `Doc not found: ${slug}`);
	}

	const html = await renderMarkdown(source);
	return {
		slug,
		title: titleFromSlug(slug),
		html
	};
};
