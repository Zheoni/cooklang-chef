import { derived } from 'svelte/store';
import { persistent } from './persistent';
import { useMediaQuery } from './useMediaQuery';
import { browser } from '$app/environment';

export type ColorScheme = 'light' | 'dark';
export type AdjustedColorScheme = ColorScheme | 'system';

const adjustedColorScheme = persistent<AdjustedColorScheme>('system', 'colorScheme');

// system color scheme, live
const prefersDark = useMediaQuery('(prefers-color-scheme: dark)');
const systemColorScheme = derived(prefersDark, ($prefersDark) => {
	return ($prefersDark ? 'dark' : 'light') satisfies ColorScheme as ColorScheme;
});

const colorScheme = derived(
	[adjustedColorScheme, systemColorScheme],
	([$adjustedColorScheme, $systemColorScheme]) => {
		if ($adjustedColorScheme !== 'dark' && $adjustedColorScheme !== 'light') {
			return $systemColorScheme;
		}
		return $adjustedColorScheme;
	}
);

if (browser) {
	colorScheme.subscribe((cs) => {
		const root = document.documentElement.classList;

		if (root.contains(cs)) return;

		const body = document.body.classList;
		body.add('transition-colors');
		setTimeout(() => body.remove('transition-colors'), 500);

		root.add(cs);
		if (cs === 'dark') {
			root.remove('light');
		} else {
			root.remove('dark');
		}
	});
}

export { colorScheme, systemColorScheme, adjustedColorScheme };
