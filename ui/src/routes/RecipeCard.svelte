<script lang="ts">
	import Divider from '$lib/Divider.svelte';
	import { API } from '$lib/constants';
	import twemoji from '$lib/twemoji';
	import Tag from '$lib/Tag.svelte';
	import type { Entry } from './+page';
	import { isValid } from '$lib/util';
	import ChefHat from '~icons/lucide/chef-hat';

	function unwrap<T>(val: T | null) {
		return val!;
	}

	export let entry: Entry;

	$: params = new URLSearchParams({ r: entry.path });
	$: valid = isValid(entry.metadata);
	$: href = `/recipe?${params}`;
</script>

<article
	class="block bg-base-3 hover:bg-base-4 hover:border-primary-9 transition-border-color
min-w-50 md:h-50 overflow-hidden rounded-xl border-2 border-transparent shadow-xl"
>
	<div class="flex flex-col md:flex-row h-full">
		{#if entry.images.length > 0}
			<a {href} class="flex-1">
				<figure
					class="border-primary-9 lt-md:border-b-4 overflow-hidden md:border-r-4 h-full max-h-50 md:max-h-none"
				>
					<img
						loading="lazy"
						class="hover:scale-101 h-full w-full object-cover transition-transform"
						src={`${API}/src/${entry.images[0].path}`}
						alt={entry.name}
					/>
				</figure>
			</a>
		{/if}
		<div class="flex flex-1 flex-col p-4 overflow-auto">
			<a {href} class="block">
				<h2 class="-mx-2 inline-block px-2 font-heading text-2xl">
					{entry.name}
				</h2>
			</a>
			<Divider class="mt-2 mb-4 px-1 text-xl" labelPos="right">
				{#if entry.metadata.value?.emoji}
					<span use:twemoji>
						{entry.metadata.value.emoji}
					</span>
				{/if}
			</Divider>
			{#if valid}
				{@const meta = unwrap(entry.metadata.value)}
				{#if meta.description}
					<p class="my-1 font-semibold line-clamp-3 shrink-0">{meta.description}</p>
				{/if}
				{#if meta.tags.length > 0}
					<div class="flex flex-wrap gap-2" class:mt-4={!meta.description}>
						{#each meta.tags as tag}
							<Tag text={tag} />
						{/each}
					</div>
				{/if}
				{#if meta.description === null && meta.tags.length === 0}
					<a {href} class="grow grid place-items-center text-3xl text-base-6">
						<ChefHat />
					</a>
				{/if}
			{:else}
				<p class="text-red-11">Error parsing metadata.</p>
			{/if}
		</div>
	</div>
</article>
