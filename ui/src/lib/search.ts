import { writable, type Readable, type Writable, derived } from 'svelte/store';
import type { Entry } from '../routes/+page';

export type Search = { tags: string[]; search: string | null; dir: string | null };

export interface SearchStore extends Writable<Search> {
	setUrlSearchParams: (params: URLSearchParams) => void;
	setQuery: (query: string) => void;
}

export function createSearch(initial: Search = { tags: [], search: null, dir: null }): SearchStore {
	const { subscribe, set, update } = writable<Search>(initial);

	const setUrlSearchParams = (params: URLSearchParams) => {
		set(searchFromUrlSearchParams(params));
	};

	const tagRegex = /tag:(\S+)/gi;
	const dirRegex = /dir:(([^\s"']+)|("[^"]*")|('[^']*'))/gi;
	const setQuery = (query: string) => {
		let tags: Search['tags'] = [];
		for (const match of query.matchAll(tagRegex)) {
			tags.push(match[1].toString());
		}
		const dirMatch = dirRegex.exec(query);
		const dirMatchVal = dirMatch && (dirMatch[2] || dirMatch[3] || dirMatch[4]);
		let dir: Search['dir'] = dirMatchVal;
		if (dir?.length === 0) dir = null;

		let search: Search['search'] = query.replaceAll(tagRegex, '').replaceAll(dirRegex, '').trim();
		if (search.length === 0) {
			search = null;
		}
		set({
			tags,
			dir,
			search
		});
	};

	return {
		subscribe,
		set,
		update,
		setUrlSearchParams,
		setQuery
	};
}

export function searchFromUrlSearchParams(params: URLSearchParams): Search {
	return {
		tags: params.getAll('tag'),
		dir: params.get('dir'),
		search: params.get('search')
	};
}

export function searchToUrlSearchParams(search: Search) {
	const params = new URLSearchParams();
	if (search.search) params.set('search', search.search);
	if (search.dir) params.set('dir', search.dir);
	search.tags.forEach((t) => params.append('tag', t));
	return params;
}

export function query(search: Search) {
	let q = '';
	search.tags?.forEach((t) => (q += `tag:${t} `));
	if (search.dir) {
		const dir = search.dir.includes(' ') ? `"${search.dir.replace("'", '"')}` : search.dir;
		q += `dir:${dir} `;
	}
	if (search.search) {
		q += search.search;
	}
	return q.trim();
}

export function filterData(search: SearchStore, data: Readable<Entry[] | null>) {
	return derived([search, data], ([$search, $data]) => {
		{
			if ($data === null) {
				return null;
			}
			let f = $data;
			if ($search.tags.length > 0) {
				f = f.filter((e) => {
					const entryTags = e.metadata.value?.tags;
					if (!entryTags) return false;
					const found = new Array<boolean>($search.tags.length).fill(false);

					for (let i = 0; i < $search.tags.length; ++i) {
						if (entryTags.includes($search.tags[i])) {
							found[i] = true;
						}
					}

					return found.every((f) => f);
				});
			}

			if ($search.search) {
				const lc = $search.search.toLocaleLowerCase();
				f = f.filter((e) => e.name.toLocaleLowerCase().includes(lc));
			}

			if ($search.dir) {
				const dirParts = $search.dir.split('/');
				f = f.filter((e) => {
					const parts = e.src_path.split('/');
					for (let i = 0; i < dirParts.length; ++i) {
						if (dirParts[i] !== parts[i]) {
							return false;
						}
					}
					return true;
				});
			}

			return f;
		}
	});
}
