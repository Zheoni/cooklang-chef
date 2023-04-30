<script lang="ts">
	import type { PageData } from './$types';
	import emptyCart from '$lib/assets/undraw_empty_cart.svg';
	import Search from '~icons/lucide/search';
	import { goto } from '$app/navigation';
	import Loader from '$lib/Loader.svelte';
	import FolderCard from './FolderCard.svelte';
	import RecipeCard from './RecipeCard.svelte';
	import { showFolders } from '$lib/settings';

	export let data: PageData;

	function submitSearch(ev: Event) {
		const form = ev.target as HTMLFormElement;
		const input = form['search'] as HTMLInputElement;
		const re = /tag:(\S+)/gi;
		const searchParams = new URLSearchParams();
		for (const match of input.value.matchAll(re)) {
			searchParams.append('t', match[1].toString());
		}
		const search = input.value.replaceAll(re, '').trim();
		if (search.length > 0) {
			searchParams.append('search', search);
		}
		goto('/?' + searchParams);
	}
</script>

<svelte:head><title>chef</title></svelte:head>

<form class="mb-8" action="/" on:submit|preventDefault={submitSearch}>
	<div class="focus:ring-base-7 flex justify-center">
		<input
			type="text"
			name="search"
			id="search"
			placeholder="Search"
			class="border-base-7 hover:border-base-8 focus:border-base-8 light color-base-12 focus-within:ring-base-7 z-1 rounded-bl rounded-tl border border-r-0 px-2"
		/>
		<button
			class="bg-base-3 border-base-7 hover:bg-base-4 active:bg-base-5 grid h-10 w-10 place-items-center rounded-br rounded-tr border"
		>
			<Search />
		</button>
	</div>
</form>

<div class="flex justify-end">
	<input type="checkbox" bind:checked={$showFolders} />
</div>

<div class="flex flex-wrap gap-6 items-stretch md:justify-center" data-sveltekit-preload-data="tap">
	{#await data.streamed.entries}
		<Loader />
	{:then { recipeEntries, folderEntries }}
		{#each folderEntries as folder}
			<FolderCard dir={folder.src_path} />
		{/each}

		{#each recipeEntries as recipeEntry}
			<RecipeCard entry={recipeEntry.entry} />
		{:else}
			<div class="container w-fit bg-base-3 bg-opacity-50 p-8 rounded-xl shadow-xl h-fit">
				<p class="font-bold text-center mt-4 mb-8 text-2xl">No recipes found</p>
				<img class="mx-auto max-w-sm" src={emptyCart} alt="Empty" />
			</div>
		{/each}
	{/await}
</div>
