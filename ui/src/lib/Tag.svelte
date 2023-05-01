<script lang="ts">
	import twemoji from './twemoji';

	export let text: string;
	export let color: string | null = null;
	export let emoji: string | null = null;

	export let link = true;
</script>

<svelte:element
	this={link ? 'a' : 'div'}
	data-sveltekit-preload-data="tap"
	href={link ? `/?${new URLSearchParams({ tag: text })}` : null}
	class="tag radix-solid-primary inline-flex h-6 w-auto flex-shrink-0 select-none items-center justify-center overflow-hidden text-ellipsis rounded px-2 text-sm font-bold tracking-wide"
	style:--color={color}
	on:click
	on:keydown
>
	<span class="mr-1 text-base inline-flex items-center">
		{#if emoji}
			<span use:twemoji>{emoji}</span>
		{:else}
			<div class="i-lucide-hash" />
		{/if}</span
	>
	<span class="overflow-hidden text-ellipsis whitespace-nowrap">
		{text}
	</span>
</svelte:element>

<style>
	.tag {
		-webkit-print-color-adjust: exact;
		print-color-adjust: exact;
	}
</style>
