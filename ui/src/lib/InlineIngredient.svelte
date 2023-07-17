<script lang="ts">
	import { componentHighlight, quantityHighlight } from './componentHighlight';
	import type { Ingredient } from './types';
	import { qValueFmt } from './Quantity.svelte';
	import { stepIngredientsView } from './settings';
	import { displayName } from './util';
	import { getContext } from 'svelte';
	import type { SectionContext } from '../routes/(narrow)/recipe/Section.svelte';

	export let index: number;
	export let ingredient: Ingredient;
	export let subscript: number | null = null;

	const { sectionIndex, steps } = getContext<SectionContext>('section');

	function genTooltipHtml(ingredient: Ingredient) {
		let html = '';
		if (ingredient.quantity) {
			let val = qValueFmt(ingredient.quantity.value);
			let unit = '';
			if (ingredient.quantity.unit) {
				unit = ` <em>${ingredient.quantity.unit}</em>`;
			}
			html = `<p class="font-serif text-center">${val}${unit}</p>`;
		}
		if (
			ingredient.relation.type === 'reference' &&
			ingredient.relation.reference_target !== 'ingredient'
		) {
			let pContent;
			if (ingredient.relation.reference_target === 'step') {
				const target = $steps[ingredient.relation.references_to];
				if (target.number) {
					pContent = `from step ${target.number}`;
				} else {
					pContent = 'from a previous step';
				}
			} else {
				pContent = `from section ${ingredient.relation.references_to + 1}`;
			}
			html += `<p class="font-serif font-italic text-sm">${pContent}</p>`;
		}
		return html;
	}

	$: elem = ingredient.modifiers.includes('RECIPE') ? ('a' as const) : ('span' as const);
	$: tooltipHtml = genTooltipHtml(ingredient);
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
