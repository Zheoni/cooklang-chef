<script lang="ts">
	import Quantity from '$lib/Quantity.svelte';
	import type { Item, Recipe } from '$lib/types';
	import type { SectionContext, SliceStep } from './Section.svelte';
	import Divider from '$lib/Divider.svelte';
	import { API } from '$lib/constants';
	import { stepIngredientsView } from '$lib/settings';
	import { extractOutcome, scaleOutcomeTooltip } from '$lib/scaleOutcomeTooltip';
	import InlineIngredient from '$lib/InlineIngredient.svelte';
	import IngredientStepItem from '$lib/IngredientStepItem.svelte';
	import { displayName } from '$lib/util';
	import { componentHighlight } from '$lib/componentHighlight';
	import { getContext } from 'svelte';
	import Timer from '$lib/Timer.svelte';

	export let step: SliceStep;
	export let recipe: Recipe;

	const { sectionIndex } = getContext<SectionContext>('section');

	$: items = step.step.items;

	function buildStepIngredients(recipe: Recipe, items: Item[]) {
		const stepIngredientsDedup = new Map<string, number[]>();
		for (const item of items) {
			if (item.type === 'ingredient') {
				const igr = recipe.ingredients[item.index];
				const group = stepIngredientsDedup.get(igr.name);
				if (group) {
					group.push(item.index);
				} else {
					stepIngredientsDedup.set(igr.name, [item.index]);
				}
			}
		}
		for (const [name, group] of stepIngredientsDedup.entries()) {
			const first = group[0];
			const newGroup = group.filter((i) => recipe.ingredients[i].quantity !== null);
			if (newGroup.length === 0) {
				newGroup.push(first);
			}
			stepIngredientsDedup.set(name, newGroup);
		}

		// javascript maps are guaranteed to remember insertion order, so it can be used
		// to iterate in the same order as ingredients in the step
		const stepIngredients = new Map<number, { subscript: number | null }>();
		const stepIngredientsLine = [];
		for (const item of items) {
			if (item.type === 'ingredient') {
				const igr = recipe.ingredients[item.index];
				const group = stepIngredientsDedup.get(igr.name);
				if (!group) continue;
				let subscript = null;
				if (group.length > 1) {
					const pos = group.indexOf(item.index);
					subscript = pos === -1 ? null : pos + 1;
				}
				stepIngredients.set(item.index, {
					subscript
				});
				if (group.includes(item.index)) {
					stepIngredientsLine.push({ index: item.index, subscript });
				}
			}
		}
		return { stepIngredients, stepIngredientsLine };
	}

	$: ({ stepIngredients, stepIngredientsLine } = buildStepIngredients(recipe, items));
</script>

<div
	class="flex gap-2 flex-col-reverse lg:flex-row"
	data-step-index={step.step_index}
	data-highlight-cls="highlight-step"
	id={`step-${$sectionIndex}-${step.step_index}`}
>
	<div
		class="rounded bg-base-2 border border-base-6 shadow p-4 grow flex flex-col transition-colors"
	>
		<p class="grow">
			{#each items as i}
				{#if i.type === 'text'}
					{i.value}
				{:else if i.type === 'inlineQuantity'}
					{@const q = recipe.inline_quantities[i.index]}
					<span class="text-red-11">
						<Quantity quantity={q} />
					</span>
				{:else if i.type === 'ingredient'}
					{@const entry = stepIngredients.get(i.index)}
					<!-- this should always be true -->
					{#if entry}
						<InlineIngredient
							index={i.index}
							ingredient={recipe.ingredients[i.index]}
							subscript={entry.subscript}
						/>
					{/if}
				{:else if i.type === 'cookware'}
					{@const cw = recipe.cookware[i.index]}
					<span
						class="text-yellow-11 font-semibold"
						use:componentHighlight={{
							index: i.index,
							component: cw,
							componentKind: 'cookware',
							currentSectionIndex: $sectionIndex
						}}>{displayName(cw)}</span
					>
				{:else if i.type === 'timer'}
					<Timer timer={recipe.timers[i.index]} seconds={recipe.timers_seconds[i.index]} />
				{/if}
			{/each}
		</p>
		{#if stepIngredientsLine.length > 0 && $stepIngredientsView !== 'hidden'}
			<Divider class="my-4" />
			<div class="stepIngredients" class:compact={$stepIngredientsView === 'compact'}>
				{#each stepIngredientsLine as { index, subscript }, arrIndex (index)}
					<div use:scaleOutcomeTooltip={extractOutcome(recipe, index)} class="w-fit">
						<IngredientStepItem {index} ingredient={recipe.ingredients[index]} {subscript} />
					</div>
					{#if arrIndex < stepIngredientsLine.length - 1 && $stepIngredientsView === 'compact'}
						<div class="w-2px rounded h-6 mx-2 bg-base-6" />
					{/if}
				{/each}
			</div>
		{/if}
	</div>
	{#if step.image}
		<div class="rounded overflow-hidden max-w-40%">
			<!-- svelte-ignore a11y-missing-attribute -->
			<img class="h-full w-full object-cover" src={`${API}/src/${step.image}`} />
		</div>
	{/if}
</div>

<style>
	.compact {
		--at-apply: flex flex-wrap;
	}

	:global(.highlight-step) > div {
		--at-apply: bg-primary-4;
	}
</style>
