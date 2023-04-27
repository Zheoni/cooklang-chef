<script lang="ts">
	import { is_valid } from '$lib/util';
	import type { PageData } from './$types';
	import emptyCart from '$lib/assets/undraw_empty_cart.svg';
	import { API } from '$lib/constants';
	import twemoji from '$lib/twemoji';
	import Tag from '$lib/Tag.svelte';
	import Search from '~icons/lucide/search';
	import { goto } from '$app/navigation';
	import Loader from '$lib/Loader.svelte';
	import Divider from '$lib/Divider.svelte';
	export let data: PageData;

	function unwrap<T>(val: T | null) {
		return val!;
	}

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

<div
	class="flex flex-wrap justify-items-stretch gap-6 md:justify-center"
	data-sveltekit-preload-data="tap"
>
	{#await data.streamed.entries}
		<Loader />
	{:then entries}
		{#each entries as entry}
			{@const params = new URLSearchParams({ r: entry.path })}
			{@const valid = is_valid(entry.metadata)}
			{@const href = `/recipe?${params}`}

			<article
				class="bg-base-3 hover:bg-base-4 hover:border-primary-9 transition-border-color min-w-200px min-h-150px flex max-w-xl flex-grow flex-col overflow-hidden rounded-xl border-2 border-transparent shadow-xl md:flex-grow-0 lg:flex-row"
			>
				{#if entry.images.length > 0}
					<a {href} class="flex-1">
						<figure class="border-primary-9 lt-lg:border-b-4 overflow-hidden lg:border-r-4 h-full">
							<img
								loading="lazy"
								class="hover:scale-101 h-full w-full object-cover transition-transform"
								src={`${API}/src/${entry.images[0].path}`}
								alt={entry.name}
							/>
						</figure>
					</a>
				{/if}
				<div class="flex flex-1 flex-col p-2">
					<div class="p-2">
						<a {href}>
							<h2 class="-mx-2 inline-block px-2 font-heading text-2xl hover:underline">
								{entry.name}
							</h2>
						</a>
						<Divider class="my-2 px-1 text-xl" labelPos="right">
							{#if entry.metadata.value?.emoji}
								<span use:twemoji>
									{entry.metadata.value.emoji}
								</span>
							{/if}
						</Divider>
						{#if valid}
							{@const meta = unwrap(entry.metadata.value)}
							{#if meta.description}
								<p class="my-1 font-serif">{meta.description}</p>
							{/if}
							{#if meta.tags.length > 0}
								<div class="flex flex-wrap gap-2">
									{#each meta.tags as tag}
										<Tag text={tag} />
									{/each}
								</div>
							{/if}
						{:else}
							<p>Error parsing metadata.</p>
						{/if}
					</div>
					<div class="ml-auto mt-auto self-end">
						<a {href} class="radix-solid-primary btn-square-9" class:btn-error={!valid}>
							<div class="i-lucide-forward text-2xl" aria-label="cook" />
						</a>
					</div>
				</div>
			</article>
		{:else}
			<div class="container max-w-lg bg-base-3 bg-opacity-50 p-8 rounded-xl shadow-xl h-fit">
				<p class="font-bold text-center mt-4 mb-8">No recipes found</p>
				<img src={emptyCart} alt="Empty" />
			</div>
		{/each}
	{/await}
</div>

<style>
	article h2 {
		background: transparent;
		background-position: 0 0;
		background-size: 600% 100%;
	}

	article:hover h2 {
		background-image: linear-gradient(
			90deg,
			rgba(70, 167, 88, 1) 20%,
			rgba(48, 164, 108, 1) 40%,
			rgba(5, 162, 194, 1) 55%,
			rgba(18, 165, 148, 1) 70%,
			rgba(48, 164, 108, 1) 85%,
			rgba(112, 225, 200, 1) 90%,
			rgba(70, 167, 88, 1) 95%
		);
		text-decoration-color: var(--un-preset-radix-grass9);
		--at-apply: bg-clip-text text-transparent;
	}

	@media (prefers-reduced-motion: no-preference) {
		article h2 {
			animation: move-bg 60s linear infinite;
		}
		@keyframes move-bg {
			to {
				background-position: 600% 0;
			}
		}
	}
</style>
