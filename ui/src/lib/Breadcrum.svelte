<script lang="ts">
	import { page } from '$app/stores';
	import Dir from '~icons/lucide/folder';

	export let dir: string;

	$: parts = dir.split('/');

	function dirHref(partIndex: number) {
		const path = parts.slice(0, partIndex + 1);
		const url = new URL($page.url);
		url.searchParams.set('dir', path.join('/'));
		return url.toString();
	}

	let homeHref: string;
	$: {
		const url = new URL($page.url);
		url.searchParams.delete('dir');
		homeHref = url.toString();
	}
</script>

<a href={homeHref} class="link"><Dir /></a>
{#each parts as part, i}
	<span class="m-1 font-bold font-mono text-base-11">/</span>
	<a href={dirHref(i)} class="link font-mono">{part}</a>
{/each}
