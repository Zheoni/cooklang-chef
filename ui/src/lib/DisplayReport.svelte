<svelte:options immutable={true} />

<script lang="ts">
	import Converter from 'ansi-to-html';
	import Code from '~icons/lucide/code-2';
	import FileCode from '~icons/lucide/file-code';

	import { API } from './constants';
	import { page } from '$app/stores';

	import toast from '$lib/toast';

	export let errors: string[];
	export let ansiString: string | null;
	export let srcPath: string | null;
	export let kind: 'warning' | 'error' = 'error';

	function displayReport(ansiString: string) {
		const converter = new Converter();
		return converter.toHtml(ansiString);
	}

	$: isLoopback =
		$page.url.hostname === 'localhost' ||
		$page.url.hostname === '127.0.0.1' ||
		$page.url.hostname === '[::1]';

	async function openEditor(srcPath: string, where: 'program' | 'editor') {
		const response = await fetch(
			`${API}/recipe/${where === 'editor' ? 'open_editor' : 'open'}/${srcPath}`
		);
		if (!response.ok) {
			toast.error('Could not open editor');
			console.error('Could not open editor:', response.status, response.statusText);
			return;
		}
		toast.success('Recipe opened');
	}
</script>

<div
	class="m-2 border rounded-xl"
	class:bg-red-3={kind === 'error'}
	class:border-red-6={kind === 'error'}
	class:bg-yellow-3={kind === 'warning'}
	class:border-yellow-6={kind === 'warning'}
>
	<div class="flex m-3 justify-end gap-2">
		{#if srcPath !== null}
			{@const srcPath2 = srcPath}
			{#if isLoopback}
				<button
					class="btn radix-solid-primary px-2 py-1 flex items-center gap-1"
					on:click={() => openEditor(srcPath2, 'editor')}
				>
					<Code /> Open in editor
				</button>
			{/if}
			<a
				class="btn radix-solid-primary px-2 py-1 flex items-center gap-1"
				href={`${API}/src/${srcPath}`}
				data-sveltekit-reload
				target="_blank"><FileCode /> View .cook</a
			>
		{/if}
	</div>
	{#if ansiString}
		<pre
			class="font-mono dark light:bg-base-3 dark:bg-base-1 p-2 rounded text-base-12 m-4 border border-yellow-6">{@html displayReport(
				ansiString
			)}</pre>
	{:else}
		<ul>
			{#each errors as err}
				<li>{err}</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	pre {
		line-height: normal;
	}
</style>
