<script lang="ts">
	import { componentHighlight, quantityHighlight } from './componentHighlight';
	import type { Ingredient } from './types';
	import { stepIngredientsView } from './settings';
	import { displayName } from './util';
	import { getContext } from 'svelte';
	import type { SectionContext } from './Section.svelte';

	export let index: number;
	export let ingredient: Ingredient;
	export let subscript: number | null = null;

	const { sectionIndex } = getContext<SectionContext>('section');

	$: elem = ingredient.modifiers.includes('RECIPE') ? ('a' as const) : ('span' as const);
</script>

<svelte:element
	this={elem}
	href={elem === 'a' ? `/recipe?${new URLSearchParams({ r: ingredient.name })}` : null}
	class:underline-link={elem === 'a'}
	class="text-primary-11 font-semibold inline-block"
	use:componentHighlight={{
		component: ingredient,
		index,
		componentKind: 'ingredient',
		currentSectionIndex: $sectionIndex
	}}
	use:quantityHighlight={{ index }}>{displayName(ingredient)}</svelte:element
>{#if subscript !== null && $stepIngredientsView !== 'hidden'}
	<sub>{subscript}</sub>
{/if}
