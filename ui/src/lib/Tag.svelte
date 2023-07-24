<script lang="ts">
	import TagLink from './TagLink.svelte';
	import twemoji from './twemoji';
	import { uiConfig } from './ui_config';

	export let text: string;

	export let link = true;
</script>

{#if link}
	<TagLink {text} />
{:else}
	{@const emoji = $uiConfig?.tags[text]?.emoji}
	<div
		class="tag text-primary-12 border-2 border-primary-7 bg-primary-3 hover:border-primary-8 hover:bg-primary-4
		inline-flex h-7 w-auto select-none items-center justify-center overflow-hidden text-ellipsis
		rounded px-2 font-semibold tracking-wide"
	>
		<span class="mr-1 text-base text-primary-11 inline-flex items-center">
			{#if emoji}
				<span use:twemoji>{emoji}</span>
			{:else}
				<div class="i-lucide-hash" />
			{/if}
		</span>
		<span class="overflow-hidden text-ellipsis whitespace-nowrap lowercase">
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
