<script lang="ts">
	import Quantity, { qValueFmt } from '$lib/Quantity.svelte';
	import type { Ingredient, Item, Recipe } from '$lib/types';
	import type { SliceStep } from './Section.svelte';
	import { ingredientHighlight, quantityHighlight } from '$lib/ingredientHighlight';
	import Divider from '$lib/Divider.svelte';
	import { tooltip } from 'svooltip';
	import { API } from '$lib/constants';
	import { stepIngredientsView } from '$lib/settings';

	export let step: SliceStep;
	export let recipe: Recipe;

	$: items = step.step.items;

	function buildStepIngredients(recipe: Recipe, items: Item[]) {
		const stepIngredientsDedup = new Map<string, number[]>();
		for (const item of items) {
			if (item.type === 'component' && item.value.kind === 'ingredient') {
				const igr = recipe.ingredients[item.value.index];
				const group = stepIngredientsDedup.get(igr.name);
				if (group) {
					group.push(item.value.index);
				} else {
					stepIngredientsDedup.set(igr.name, [item.value.index]);
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
		const stepIngredients = new Map<number, { ingredient: Ingredient; subscript: number | null }>();
		for (const item of items) {
			if (item.type === 'component' && item.value.kind === 'ingredient') {
				const igr = recipe.ingredients[item.value.index];
				const group = stepIngredientsDedup.get(igr.name);
				if (!group) continue;
				let subscript = null;
				if (group.length > 1) {
					const pos = group.indexOf(item.value.index);
					subscript = pos === -1 ? null : pos + 1;
				}
				stepIngredients.set(item.value.index, {
					ingredient: igr,
					subscript
				});
			}
		}
		return stepIngredients;
	}

	$: stepIngredients = buildStepIngredients(recipe, items);
	$: stepIngredientsArray = Array.from(stepIngredients.entries()); // TODO unnecesary copy
</script>

<div class="flex gap-2 flex-col-reverse lg:flex-row">
	<div class="rounded bg-base-2 border border-base-6 shadow p-4 grow flex flex-col">
		<p class="grow">
			{#each items as i}
				{#if i.type === 'text'}
					{i.value}
				{:else if i.type === 'inlineQuantity'}
					{@const q = recipe.inline_quantities[i.value]}
					<span class="text-red-11">
						<Quantity quantity={q} />
					</span>
				{:else if i.type === 'component'}
					{@const component = i.value}
					{#if component.kind === 'ingredient'}
						{@const entry = stepIngredients.get(component.index)}
						<!-- ? this should always be true -->
						{#if entry}
							<span
								class="text-primary-11 font-semibold inline-block"
								use:ingredientHighlight={{ ingredient: entry.ingredient, index: component.index }}
								use:quantityHighlight={{ index: component.index }}
								use:tooltip={{
									content: entry.ingredient.quantity
										? `<span class="font-serif">${qValueFmt(entry.ingredient.quantity.value)} <em>${
												entry.ingredient.quantity.unit
										  }</em></span>`
										: '',
									html: true,
									visibility:
										entry.ingredient.quantity !== null && $stepIngredientsView === 'hidden'
								}}>{entry.ingredient.alias ?? entry.ingredient.name}</span
							>{#if entry.subscript && $stepIngredientsView !== 'hidden'}
								<sub>{entry.subscript}</sub>
							{/if}
						{/if}
					{:else if component.kind === 'cookware'}
						{@const cw = recipe.cookware[component.index]}
						<span class="text-yellow-11 font-semibold">{cw.name}</span>
					{:else if component.kind === 'timer'}
						{@const tm = recipe.timers[component.index]}
						<span class="text-indigo-11 font-semibold"
							>{tm.name ?? ''}<Quantity quantity={tm.quantity} /></span
						>
					{/if}
				{/if}
			{/each}
		</p>
		{#if stepIngredientsArray.length > 0 && $stepIngredientsView !== 'hidden'}
			<Divider class="my-4" />
			<div class="stepIngredients" class:compact={$stepIngredientsView === 'compact'}>
				{#each stepIngredientsArray as [index, { ingredient, subscript }], arrIndex (index)}
					<div>
						<span use:quantityHighlight={{ index }} data-highlight-cls="qhighlight"
							>{ingredient.alias ?? ingredient.name}</span
						>{#if subscript}
							<sub>{subscript}</sub>
						{/if}{#if ingredient.modifiers.includes('OPT')}
							(opt)
						{/if}{#if ingredient.quantity}
							: <span class="text-base-11"><Quantity quantity={ingredient.quantity} /></span>
						{/if}
					</div>
					{#if arrIndex < stepIngredientsArray.length - 1 && $stepIngredientsView === 'compact'}
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
	.stepIngredients :global(.qhighlight) {
		--at-apply: underline text-primary-11 transition-colors;
	}

	.compact {
		--at-apply: flex flex-wrap;
	}
</style>
