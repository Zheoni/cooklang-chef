import { writable, get, derived } from 'svelte/store';

// On-demand locale loading
export type Loader = () => Promise<any>;
export type Lang = {
	loader: Loader;
	emoji: string;
	name: string;
};
export const localesData = writable<Record<string, Lang>>({});

export function register(code: string, lang: Lang) {
	localesData.update((loaders) => {
		loaders[code] = lang;
		return loaders;
	});
}

export async function load(code: string) {
	console.log('loading locale', code);
	return get(localesData)[code].loader();
}

export const locales = derived(localesData, ($loaders) => Object.keys($loaders));
