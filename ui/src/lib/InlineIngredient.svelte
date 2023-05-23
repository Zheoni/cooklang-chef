<script lang="ts">
	import { tooltip } from 'svooltip';
	import { componentHighlight, quantityHighlight } from './componentHighlight';
	import type { Ingredient } from './types';
	import { qValueFmt } from './Quantity.svelte';
	import { stepIngredientsView } from './settings';
	import { displayName } from './util';

	export let index: number;
	export let ingredient: Ingredient;
	export let subscript: number | null = null;

	$: elem = ingredient.modifiers.includes('RECIPE') ? ('a' as const) : ('span' as const);
</script>

<svelte:element
	this={elem}
	href={elem === 'a' ? `/recipe?${new URLSearchParams({ r: ingredient.name })}` : null}
	class:underline-link={elem === 'a'}
	class="text-primary-11 font-semibold inline-block"
	use:componentHighlight={{ component: ingredient, index, componentKind: 'ingredient' }}
	use:quantityHighlight={{ index }}
	use:tooltip={{
		content: ingredient.quantity
			? `<span class="font-serif">${qValueFmt(ingredient.quantity.value)} <em>${
					ingredient.quantity.unit
			  }</em></span>`
			: '',
		html: true,
		visibility: ingredient.quantity !== null && $stepIngredientsView === 'hidden'
	}}>{displayName(ingredient)}</svelte:element
>{#if subscript !== null && $stepIngredientsView !== 'hidden'}
	<sub>{subscript}</sub>
{/if}
