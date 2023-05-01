<svelte:options immutable={true} />

<script lang="ts">
	import Converter from 'ansi-to-html';
	import FileCode from '~icons/lucide/file-code';

	import { API } from './constants';
	import OpenInEditor from './OpenInEditor.svelte';

	export let errors: string[];
	export let ansiString: string | null;
	export let srcPath: string | null;
	export let kind: 'warning' | 'error' = 'error';

	function displayReport(ansiString: string) {
		const converter = new Converter();
		return converter.toHtml(ansiString);
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
			<OpenInEditor {srcPath} />
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
