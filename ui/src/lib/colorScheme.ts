import { derived } from 'svelte/store';
import { persistent } from './persistent';
import { useMediaQuery } from './useMediaQuery';
import { browser } from '$app/environment';
import { tick } from 'svelte';

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
		return resolveAdjustedColors($adjustedColorScheme, $systemColorScheme);
	}
);

export function resolveAdjustedColors(adjusted: AdjustedColorScheme, system: ColorScheme) {
	if (adjusted === 'system') {
		return system;
	}
	return adjusted;
}

if (browser) {
	colorScheme.subscribe((cs) => {
		const root = document.documentElement.classList;
		root.add('change-theme');
		setTimeout(() => root.remove('change-theme'), 1000);
		root.add(cs);
		if (cs === 'dark') {
			root.remove('light');
		} else {
			root.remove('dark');
		}
	});
}

export { colorScheme, systemColorScheme, adjustedColorScheme };
