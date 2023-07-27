import { browser } from '$app/environment';
import { defaultLocale, loadLocale, resolve } from '$lib/i18n';
import { redirect } from '@sveltejs/kit';

export const ssr = false;
export const prerender = false;

export async function load({ params }) {
	let initLocale: string | null = null;
	if (params.lang) {
		initLocale = resolve(params.lang);
		if (initLocale === null) throw redirect(303, '/');
	} else if (browser) {
		initLocale = resolve(window.navigator.language);
	}
	await loadLocale(initLocale ?? defaultLocale);
}
