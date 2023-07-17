<script lang="ts">
	import { API } from '$lib/constants';
	import { getContext } from 'svelte';
	import type { SectionContext, SliceStep } from './Section.svelte';

	export let step: SliceStep;

	const { sectionIndex } = getContext<SectionContext>('section');
</script>

<div
	class="flex gap-2 items-center flex-col-reverse lg:flex-row my-8 text-step"
	id={`step-${$sectionIndex}-${step.step_index}`}
	data-step-index={step.step_index}
	data-highlight-cls="highlight-text-step"
>
	<p class="grow indent-4">
		{#each step.step.items as item}
			{#if item.type === 'text'}
				{item.value}
			{/if}
		{/each}
	</p>
	{#if step.image}
		<div class="rounded overflow-hidden max-w-40%">
			<!-- svelte-ignore a11y-missing-attribute -->
			<img class="h-full w-full object-cover" src={`${API}/src/${step.image}`} />
		</div>
	{/if}
</div>

<style>
	.text-step {
		transition: background-color 150ms, box-shadow 150ms;
	}

	:global(.highlight-text-step) {
		--at-apply: bg-primary-2;
		box-shadow: 0 0 10px 10px var(--un-preset-radix-grass2);
	}
</style>
