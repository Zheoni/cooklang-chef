import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { API } from '$lib/constants';
import type { Image, Metadata, Report } from '$lib/types';

export type Entry = {
	name: string;
	path: string;
	metadata: Report<Metadata>;
	src_path: string;
	images: Image[];
};

export const load = (({ fetch, url }) => {
	const apiUrl = `${API}/recipe/metadata`;
	try {
		const entries = fetch(apiUrl).then((r) => r.json() as Promise<Entry[]>);
		return { streamed: { entries } };
	} catch (_e) {
		throw error(400);
	}
}) satisfies PageLoad;
