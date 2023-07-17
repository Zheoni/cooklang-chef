<script lang="ts">
	import Divider from '$lib/Divider.svelte';
	import { API } from '$lib/constants';
	import twemoji from '$lib/twemoji';
	import Tag from '$lib/Tag.svelte';
	import Card from './Card.svelte';
	import type { Entry } from './+page';
	import { isValid } from '$lib/util';

	function unwrap<T>(val: T | null) {
		return val!;
	}

	export let entry: Entry;

	$: params = new URLSearchParams({ r: entry.path });
	$: valid = isValid(entry.metadata);
	$: href = `/recipe?${params}`;
</script>

<Card as="article">
	<div class="flex flex-col lg:flex-row h-full">
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
					<h2 class="-mx-2 inline-block px-2 font-heading text-2xl">
						{entry.name}
					</h2>
				</a>
				<Divider class="mt-2 mb-3 px-1 text-xl" labelPos="right">
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
	</div>
</Card>
