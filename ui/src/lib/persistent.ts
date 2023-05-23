import { writable, type Writable } from 'svelte/store';
import { browser } from '$app/environment';

export function persistent(initial: string, storeKey: string): Writable<string>;
export function persistent<T>(initial: T, storeKey: string): Writable<T>;
export function persistent(initial: any, storeKey: string) {
	let store;
	if (typeof initial === 'string') {
		store = persistentImpl(
			initial,
			storeKey,
			(s) => s,
			(v) => v
		);
	} else {
		store = persistentImpl(
			initial,
			storeKey,
			(s) => JSON.parse(s),
			(v) => JSON.stringify(v)
		);
	}
	return store;
}

function persistentImpl<T>(
	initial: T,
	storeKey: string,
	load: (s: string) => T,
	save: (v: T) => string
) {
	let _initial: T | undefined = initial;
	if (browser) {
		const stored = localStorage.getItem(storeKey);
		if (stored !== null) {
			try {
				_initial = load(stored);
			} catch (error) {
				console.warn(error);
			}
		}
	}
	const store = writable<T>(_initial);
	if (browser) {
		store.subscribe((value) => {
			localStorage.setItem(storeKey, save(value));
		});
	}
	return store;
}
