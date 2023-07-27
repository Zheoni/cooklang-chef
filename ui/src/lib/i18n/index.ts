import { register, load, locales, localesData } from './load';
export { locales as allLocales, localesData };

import rosetta from 'rosetta';
import { derived, get, writable } from 'svelte/store';

register('en', { loader: () => import('./en.json'), emoji: '🇬🇧', name: 'English' });
register('es', { loader: () => import('./es.json'), emoji: '🇪🇸', name: 'Español' });

const i18n = rosetta();

export const defaultLocale = 'en';

export const locale = writable<string>();
export const localeReady = writable(false);

export async function loadLocale(loc: string) {
	locale.set(loc);
	let unsub;
	await new Promise((res) => {
		const store = derived([locale, localeReady], ([$locale, $ready]) => {
			if ($locale === loc && $ready) res(true);
		});

		unsub = store.subscribe((ready) => {
			if (ready) res(true);
		});
	});
	unsub!();
}

locale.subscribe(async (loc) => {
	if (!loc) return;
	const lang = (loc && resolve(loc)) ?? defaultLocale;

	if (i18n.table(lang) === undefined) {
		localeReady.set(false);
		const d = await load(lang);
		i18n.set(lang, d);
		localeReady.set(true);
	}

	i18n.locale(lang);
});

// Force an update wherever the function is used
export const t = derived([locale, localeReady], ([$lang, _$ready]) => {
	return (key: string, params?: Record<string, any>) => i18n.t(key, params, $lang);
});

export function resolve(code: string) {
	const all = get(locales);

	if (all.includes(code)) return code;
	for (const known of all) {
		if (code.startsWith(known)) return known;
	}
	return null;
}
