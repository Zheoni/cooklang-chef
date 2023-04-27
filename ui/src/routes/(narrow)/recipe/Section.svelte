<script lang="ts" context="module">
	export type SliceStep = {
		step: Step;
		step_index: number;
		step_count: number | null;
		image: string | null;
	};
</script>

<script lang="ts">
	import type { Image, Recipe, Section, Step } from '$lib/types';
	import RegularStep from './RegularStep.svelte';
	import TextStep from './TextStep.svelte';

	export let section: Section;
	export let section_index: number;
	export let recipe: Recipe;
	export let images: Image[];

	type Slice = {
		is_text: boolean;
		steps: SliceStep[];
		step_count: number | null;
	};

	function buildSlices(section: Section) {
		let slices: Slice[] = [];
		let count = 1;
		for (let step_index = 0; step_index < section.steps.length; step_index += 1) {
			const step = section.steps[step_index];
			const image =
				images.find(
					(im) => im.indexes && im.indexes[0] === section_index && im.indexes[1] === step_index
				)?.path ?? null;
			const sliceStep: SliceStep = {
				step,
				step_index,
				step_count: step.is_text ? null : count++,
				image
			};
			const lastSlice = slices[slices.length - 1];
			if (lastSlice === undefined) {
				slices.push({
					is_text: step.is_text,
					steps: [sliceStep],
					step_count: sliceStep.step_count
				});
				continue;
			}
			if (lastSlice.is_text === step.is_text) {
				lastSlice.steps.push(sliceStep);
			} else {
				slices.push({
					is_text: step.is_text,
					steps: [sliceStep],
					step_count: sliceStep.step_count
				});
			}
		}
		return slices;
	}
	$: slices = buildSlices(section);
</script>

{#if section.name}
	<h2 class="text-xl font-semibold">{section.name}</h2>
{:else if section_index > 0}
	<h2 class="text-xl font-semibold">Section {section_index + 1}</h2>
{/if}

{#each slices as slice}
	{#if slice.is_text}
		{#each slice.steps as step}
			<TextStep {step} />
		{/each}
	{:else}
		<ol start={slice.step_count || 1} class="list-decimal ms-6">
			{#each slice.steps as step}
				<li class="mb-8">
					<RegularStep {step} {recipe} />
				</li>
			{/each}
		</ol>
	{/if}
{/each}
