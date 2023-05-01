<script lang="ts">
	import type { PageData } from './$types';
	import emptyCart from '$lib/assets/undraw_empty_cart.svg';
	import SearchIcon from '~icons/lucide/search';
	import { goto } from '$app/navigation';
	import Loader from '$lib/Loader.svelte';
	import FolderCard from './FolderCard.svelte';
	import RecipeCard from './RecipeCard.svelte';
	import { showFolders } from '$lib/settings';
	import type { Entry } from './+page';
	import { derived, get, writable } from 'svelte/store';
	import { page } from '$app/stores';
	import {
		createSearch,
		filterData,
		query,
		searchToUrlSearchParams,
		type Search
	} from '$lib/search';
	import { browser } from '$app/environment';

	// Chained stores for search may lead to too much memory usage ?

	export let data: PageData;
	const dataStore = writable<Entry[] | null>(null);
	$: data.streamed.entries.then(dataStore.set);

	const search = createSearch();
	const searchQuery = writable('');
	$: search.setUrlSearchParams($page.url.searchParams);
	$: searchQuery.set(query($search));
	$: search.setQuery($searchQuery);
	$: {
		if ($showFolders === false) {
			search.update((s) => {
				s.dir = null;
				return s;
			});
		}
	}
	let lastTimeout: number | undefined = undefined;
	$: {
		if (lastTimeout) clearTimeout(lastTimeout);
		if (browser) {
			lastTimeout = setTimeout(updateUrl, 1000, $search);
		}
	}

	function updateUrl(search: Search) {
		const params = searchToUrlSearchParams(search);
		goto(`/?${params}`, {
			replaceState: true,
			noScroll: true,
			keepFocus: true
		});
	}

	function submitSearch() {
		const params = searchToUrlSearchParams(get(search));
		goto(`/?${params}`);
	}

	const filtered = filterData(search, dataStore);
	const splitted = derived([filtered, showFolders], ([$filtered, $showFolders]) => {
		if ($filtered === null) return null;
		if ($showFolders) {
			return splitEntries($filtered);
		} else {
			return $filtered.map((e) => ({ type: 'recipe', entry: e })) as RecipeOrFolder[];
		}
	});

	type RecipeOrFolder =
		| {
				type: 'recipe';
				entry: Entry;
		  }
		| {
				type: 'folder';
				src_path: string;
		  };

	function splitEntries(entries: Entry[]): RecipeOrFolder[] {
		const dir = $search.dir ?? '';

		const recipes: RecipeOrFolder[] = [];

		const dirParts = dir?.split('/').filter((p) => p.length > 0) ?? [];
		const dirs = new Set<string>();
		for (const entry of entries) {
			const parts = entry.src_path.split('/');
			const restParts = parts.slice(dirParts.length);
			if (restParts.length === 1 && restParts[0].endsWith('.cook')) {
				recipes.push({ type: 'recipe', entry });
			} else {
				const src_path = dirParts.concat(restParts[0]).join('/');
				dirs.add(src_path);
			}
		}

		const folders: RecipeOrFolder[] = [];
		for (const dir of dirs) {
			folders.push({ type: 'folder', src_path: dir });
		}
		return folders.concat(recipes);
	}
</script>

<svelte:head><title>chef</title></svelte:head>

<form
	class="mb-8 flex gap-4 items-center justify-center"
	action="/"
	on:submit|preventDefault={submitSearch}
>
	<div class="focus:ring-base-7 flex justify-center">
		<input
			type="text"
			name="search"
			id="search"
			placeholder="Search"
			autocomplete="off"
			class="border-base-7 hover:border-base-8 focus:border-base-8 light color-base-12 focus-within:ring-base-7 z-1 rounded-bl rounded-tl border border-r-0 px-2"
			bind:value={$searchQuery}
			on:blur={() => updateUrl($search)}
		/>
		<button
			class="bg-base-3 border-base-7 hover:bg-base-4 active:bg-base-5 grid h-10 w-10 place-items-center rounded-br rounded-tr border"
		>
			<SearchIcon />
		</button>
	</div>

	<div>
		<input type="checkbox" id="showFolders" bind:checked={$showFolders} />
		<label for="showFolders">Show folders</label>
	</div>
</form>

<div class="flex flex-wrap gap-6 items-stretch md:justify-center" data-sveltekit-preload-data="tap">
	{#if $splitted === null}
		<Loader />
	{:else}
		{#each $splitted as recipeOrFolder}
			{#if recipeOrFolder.type === 'folder'}
				<FolderCard dir={recipeOrFolder.src_path} />
			{:else}
				<RecipeCard entry={recipeOrFolder.entry} />
			{/if}
		{:else}
			<div class="container w-fit bg-base-3 bg-opacity-50 p-8 rounded-xl shadow-xl h-fit">
				<p class="font-bold text-center mt-4 mb-8 text-2xl">No recipes found</p>
				<img class="mx-auto max-w-sm" src={emptyCart} alt="Empty" />
			</div>
		{/each}
	{/if}
</div>
