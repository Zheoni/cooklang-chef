<script lang="ts">
	import type { PageData } from './$types';
	import emptyCart from '$lib/assets/undraw_empty_cart.svg';
	import SearchIcon from '~icons/lucide/search';
	import { goto } from '$app/navigation';
	import Loader from '$lib/Loader.svelte';
	import FolderCard from './FolderCard.svelte';
	import RecipeCard from './RecipeCard.svelte';
	import { showFolders as divideInFolders } from '$lib/settings';
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
	import Divider from '$lib/Divider.svelte';
	import Breadcrum from '$lib/Breadcrum.svelte';
	import { t } from '$lib/i18n';

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
		if ($divideInFolders === false) {
			search.update((s) => {
				s.dir = null;
				return s;
			});
		}
	}
	let lastTimeout: number | undefined = undefined;
	let first = true;
	$: {
		if (lastTimeout) clearTimeout(lastTimeout);
		if (browser && !first) {
			lastTimeout = setTimeout(updateUrl, 1000, $search);
		}
		first = false;
	}

	function updateUrl(search: Search) {
		const params = searchToUrlSearchParams(search).toString();
		let url = window.location.pathname;
		if (params.length > 0) {
			url += '?' + params;
		}
		goto(url, {
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
	const splitted = derived([filtered, divideInFolders], ([$filtered, $showFolders]) => {
		if ($filtered === null) return null;
		if ($showFolders) {
			return splitEntries($filtered);
		} else {
			return { recipes: $filtered, folders: [] };
		}
	});

	function splitEntries(entries: Entry[]) {
		const dir = $search.dir ?? '';

		const recipes = [];

		const dirParts = dir?.split('/').filter((p) => p.length > 0) ?? [];
		const dirs = new Set<string>();
		for (const entry of entries) {
			const parts = entry.src_path.split('/');
			const restParts = parts.slice(dirParts.length);
			if (restParts.length === 1 && restParts[0].endsWith('.cook')) {
				recipes.push(entry);
			} else {
				const src_path = dirParts.concat(restParts[0]).join('/');
				dirs.add(src_path);
			}
		}

		const folders = [];
		for (const dir of dirs) {
			folders.push(dir);
		}
		return { recipes, folders };
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key.toLowerCase() == 's') {
			document.getElementById('search')?.focus();
			e.preventDefault();
		}
	}
</script>

<svelte:head><title>chef</title></svelte:head>

<svelte:document on:keydown={handleKeydown} />

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
			placeholder={$t('index.search')}
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
		<input type="checkbox" id="divideInFolders" bind:checked={$divideInFolders} />
		<label for="divideInFolders">{$t('index.divideInFolders')}</label>
	</div>
</form>

{#if $divideInFolders}
	<div class="m-4">
		<Breadcrum dir={$search.dir ?? ''} />
	</div>
{/if}

{#if $splitted === null}
	<div class="mx-auto grid place-items-center">
		<Loader />
	</div>
{:else}
	<div class="flex flex-col sm:flex-row flex-wrap gap-6">
		{#each $splitted.folders as folder}
			<FolderCard dir={folder} />
		{/each}
	</div>
	{#if $splitted.folders.length > 0}
		<Divider class="m-4" />
	{/if}
	<div
		class="grid grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3 gap-6 justify-items-stretch items-stretch"
		data-sveltekit-preload-data="tap"
	>
		{#each $splitted.recipes as recipe}
			<RecipeCard entry={recipe} />
		{/each}
	</div>
	{#if $splitted.recipes.length === 0}
		<div
			class="container w-fit bg-base-3 bg-opacity-50 p-8 rounded-xl shadow-xl h-fit mx-auto my-4"
		>
			<p class="font-bold text-center mt-4 mb-8 text-2xl">{$t('index.noRecipes')}</p>
			<img class="mx-auto max-w-sm" src={emptyCart} alt="Empty" />
		</div>
	{/if}
{/if}
