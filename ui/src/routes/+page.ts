import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { API } from '$lib/constants';
import type { Image, Metadata, Report } from '$lib/types';
import { showFolders } from '$lib/settings';
import { get } from 'svelte/store';

export type Entry = {
	name: string;
	path: string;
	metadata: Report<Metadata>;
	src_path: string;
	images: Image[];
};

export type RecipeEntry = {
	type: 'recipe';
	entry: Entry;
};
export type FolderEntry = {
	type: 'folder';
	src_path: string;
};
export type RecipeOrFolder = RecipeEntry | FolderEntry;

export const load = (({ fetch, url }) => {
	const apiUrl = `${API}/recipe/metadata`;
	try {
		const entries = fetch(apiUrl)
			.then((r) => r.json() as Promise<Entry[]>)
			.then((entries) => {
				const tag = url.searchParams.get('t');
				if (tag) {
					entries = entries.filter((e) => e.metadata.value?.tags.includes(tag));
				}
				const search = url.searchParams.get('search')?.toLocaleLowerCase();
				if (search) {
					entries = entries.filter((e) => e.name.toLocaleLowerCase().includes(search));
				}

				let recipeEntries: RecipeEntry[] = [];
				let folderEntries: FolderEntry[] = [];
				const dir = url.searchParams.get('dir');
				if (dir || (get(showFolders) && !tag && !search)) {
					if (dir) {
						entries = entries.filter((e) => e.src_path.startsWith(dir));
					}
					const dirParts = dir?.split('/') ?? [];
					const dirs = new Set<string>();
					for (const entry of entries) {
						const rest = dir ? entry.src_path.slice(dir.length) : entry.src_path;
						const parts = rest.split('/').filter((p) => p.length > 0);
						if (parts.length === 1 && parts[0].endsWith('.cook')) {
							recipeEntries.push({ type: 'recipe', entry });
						} else {
							const src_path = entry.src_path
								.split('/')
								.slice(0, dirParts.length + 1)
								.join('/');
							dirs.add(src_path);
						}
					}
					for (const dir of dirs) {
						folderEntries.push({ type: 'folder', src_path: dir });
					}
				} else {
					recipeEntries = entries.map((e) => ({ type: 'recipe', entry: e }));
				}

				return { recipeEntries, folderEntries };
			});

		return { streamed: { entries } };
	} catch (_e) {
		throw error(400);
	}
}) satisfies PageLoad;
