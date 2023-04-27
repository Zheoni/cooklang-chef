/// <reference types="@sveltejs/kit" />
/// <reference types="unplugin-icons/types/svelte" />

import type { Image, ReportError } from '$lib/types';

// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		interface Error {
			code?: ErrorCode;
			srcPath?: string;
			images?: Image[];
			parse_errors?: string[];
			parse_warnings?: string[];
			fancy_report?: string | null;
		}

		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
		type ErrorCode = 'PARSE';
	}

	namespace svelteHTML {
		interface HTMLAttributes<T> {
			'on:outclick'?: (event: CustomEvent) => void;
		}
	}
}

export {};
