<script lang="ts" context="module">
	export type SectionContext = {
		sectionIndex: Readable<number>;
		content: Readable<Content[]>;
	};
</script>

<script lang="ts">
	import type { Content, Image, Recipe, Section, Step } from '$lib/types';
	import { setContext } from 'svelte';
	import RegularStep from './RegularStep.svelte';
	import { readonly, writable, type Readable } from 'svelte/store';

	export let section: Section;
	export let section_index: number;
	export let recipe: Recipe;
	export let images: Image[];

	const sectionIndexStore = writable(section_index);
	const contentStore = writable(section.content);
	$: sectionIndexStore.set(section_index);
	$: contentStore.set(section.content);
	setContext<SectionContext>('section', {
		sectionIndex: readonly(sectionIndexStore),
		content: readonly(contentStore)
	});
</script>

<div
	data-section-index={section_index}
	id={`section-${section_index}`}
	data-highlight-cls="highlight-section"
	class="section bg-transparent transition-colors"
>
	{#if section.name}
		<h2 class="text-2xl my-3 font-semibold">{section.name}</h2>
	{:else if section_index > 0}
		<h2 class="text-2xl my-3 font-semibold">Section {section_index + 1}</h2>
	{/if}

	{#each section.content as content, index}
		{#if content.type === 'step'}
			{@const step = content.value}
			<div class="flex my-7">
				<span class="mx-1 mt-2">{content.value.number}.</span>
				<RegularStep
					step={content.value}
					stepIndex={index}
					image={images.find(
						(im) =>
							im.indexes &&
							im.indexes.section === section_index &&
							im.indexes.step === step.number - 1
					)?.path}
					{recipe}
				/>
			</div>
		{:else if content.type === 'text'}
			<p class="grow indent-4">
				{content.value}
			</p>
		{/if}
	{/each}
</div>

<style>
	.section {
		transition: background-color 150ms, box-shadow 150ms;
	}

	:global(.highlight-section) {
		--at-apply: bg-primary-2;
		box-shadow: 0 0 10px 10px var(--un-preset-radix-grass2);
	}
</style>
