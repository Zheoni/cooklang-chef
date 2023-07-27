<script lang="ts" context="module">
	export type Slice = {
		isText: boolean;
		steps: SliceStep[];
	};

	export type SliceStep = {
		step: Step;
		// the index in the array
		step_index: number;
		image: string | null;
	};

	export type SectionContext = {
		sectionIndex: Readable<number>;
		steps: Readable<Step[]>;
	};
</script>

<script lang="ts">
	import type { Image, Recipe, Section, Step } from '$lib/types';
	import { setContext } from 'svelte';
	import RegularStep from './RegularStep.svelte';
	import TextStep from './TextStep.svelte';
	import { readonly, writable, type Readable } from 'svelte/store';
	import { isTextStep } from '$lib/util';

	export let section: Section;
	export let section_index: number;
	export let recipe: Recipe;
	export let images: Image[];

	function buildSlices(section: Section) {
		let slices: Slice[] = [];
		for (let step_index = 0; step_index < section.steps.length; step_index += 1) {
			const step = section.steps[step_index];
			const image =
				images.find(
					(im) =>
						im.indexes && im.indexes.section === section_index && im.indexes.step === step_index
				)?.path ?? null;
			const sliceStep: SliceStep = {
				step,
				step_index,
				image
			};
			const lastSlice = slices[slices.length - 1];
			if (lastSlice === undefined) {
				slices.push({
					isText: isTextStep(step),
					steps: [sliceStep]
				});
				continue;
			}
			if (lastSlice.isText === isTextStep(step)) {
				lastSlice.steps.push(sliceStep);
			} else {
				slices.push({
					isText: isTextStep(step),
					steps: [sliceStep]
				});
			}
		}
		return slices;
	}
	$: slices = buildSlices(section);

	const sectionIndexStore = writable(section_index);
	$: sectionIndexStore.set(section_index);
	const stepsStore = writable(section.steps);
	$: stepsStore.set(section.steps);
	setContext<SectionContext>('section', {
		sectionIndex: readonly(sectionIndexStore),
		steps: readonly(stepsStore)
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

	{#each slices as slice}
		{#if slice.isText}
			{#each slice.steps as step}
				<TextStep {step} />
			{/each}
		{:else}
			<ol start={slice.steps[0].step.number ?? 1} class="list-decimal ms-6">
				{#each slice.steps as step}
					<li class="mb-8" value={step.step.number}>
						<RegularStep {step} {recipe} />
					</li>
				{/each}
			</ol>
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
