export type DocNavItem = {
	slug: string;
	title: string;
	section?: string;
};

/** Product-facing docs mirrored from docs/ (excludes docs/internal). */
export const DOC_NAV: DocNavItem[] = [
	{ slug: 'why-grapheme', title: 'Why Grapheme', section: 'Start' },
	{ slug: 'quickstart', title: 'Quickstart', section: 'Start' },
	{ slug: 'hero-workflow', title: 'Hero workflow', section: 'Start' },
	{ slug: 'language-tour', title: 'Language tour', section: 'Language' },
	{ slug: 'playbooks', title: 'Playbooks', section: 'Language' },
	{ slug: 'faq', title: 'FAQ', section: 'Language' },
	{ slug: 'tutorials', title: 'Tutorials', section: 'Learn' },
	{ slug: 'tutorials/01-first-value', title: '01 · First value', section: 'Learn' },
	{ slug: 'tutorials/02-language-core', title: '02 · Language core', section: 'Learn' },
	{ slug: 'tutorials/03-control-flow-and-state', title: '03 · Control flow', section: 'Learn' },
	{ slug: 'tutorials/04-integrations-and-policy', title: '04 · Integrations', section: 'Learn' },
	{ slug: 'tutorials/05-debugging-and-operations', title: '05 · Debugging', section: 'Learn' },
	{ slug: 'tutorials/06-realworld-scenarios', title: '06 · Real-world', section: 'Learn' },
	{ slug: 'tutorials/07-adoption-playbook', title: '07 · Adoption', section: 'Learn' },
	{ slug: 'tutorials/08-weekly-kpi-build-along', title: '08 · Weekly KPI', section: 'Learn' },
	{ slug: 'tutorials/09-support-triage-build-along', title: '09 · Support triage', section: 'Learn' },
	{ slug: 'tutorials/10-invoice-intake-build-along', title: '10 · Invoice intake', section: 'Learn' }
];

export function titleFromSlug(slug: string): string {
	const hit = DOC_NAV.find((d) => d.slug === slug);
	if (hit) return hit.title;
	return slug
		.split('/')
		.pop()!
		.replace(/-/g, ' ')
		.replace(/\b\w/g, (c) => c.toUpperCase());
}
