<script lang="ts">
	import TagLink from './TagLink.svelte';
	import twemoji from './twemoji';

	export let text: string;
	export let color: string | null = null;
	export let emoji: string | null = null;

	export let link = true;
</script>

{#if link}
	<TagLink {text} {color} {emoji} />
{:else}
	<div
		class="tag radix-solid-primary inline-flex h-6 w-auto flex-shrink-0 select-none items-center justify-center overflow-hidden text-ellipsis rounded px-2 text-sm font-bold tracking-wide"
	>
		<span class="mr-1 text-base inline-flex items-center">
			{#if emoji}
				<span use:twemoji>{emoji}</span>
			{:else}
				<div class="i-lucide-hash" />
			{/if}
		</span>
		<span class="overflow-hidden text-ellipsis whitespace-nowrap">
			{text}
		</span>
	</div>
{/if}

<style>
	.tag {
		-webkit-print-color-adjust: exact;
		print-color-adjust: exact;
	}
</style>
