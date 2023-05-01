import { readable } from 'svelte/store';
import { browser } from '$app/environment';
//export a function that return a readable given a string media query as input
export function useMediaQuery(mediaQueryString: string) {
	//we inizialize the readable as null and get the callback with the set function
	const matches = readable<boolean | null>(null, (set) => {
		// skip this in the server-side
		if (!browser) {
			return;
		}
		//we match the media query
		const m = window.matchMedia(mediaQueryString);
		//we set the value of the reader to the matches property
		set(m.matches);
		//we create the event listener that will set the new value on change
		const listener = (event: MediaQueryListEvent) => set(event.matches);
		//we add the new event listener
		m.addEventListener('change', listener);
		//we return the stop function that will clean the event listener
		return () => {
			m.removeEventListener('change', listener);
		};
	});
	//then we return the readable
	return matches;
}
