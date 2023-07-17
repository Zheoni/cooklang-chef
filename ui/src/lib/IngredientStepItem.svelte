<svelte:options immutable={true} />

<script lang="ts">
	import { getContext } from 'svelte';
	import Quantity from './Quantity.svelte';
	import { quantityHighlight, intermediateIgrHighlight } from './componentHighlight';
	import type { Ingredient, Step } from './types';
	import { displayName } from './util';
	import type { SectionContext } from '../routes/(narrow)/recipe/Section.svelte';

	export let index: number;
	export let ingredient: Ingredient;
	export let subscript: number | null = null;

	const { sectionIndex, steps } = getContext<SectionContext>('section');

	function getStepName(step: Step) {
		if (step.number !== null) {
			return 'step ' + step.number;
		} else {
			return 'a previous step';
		}
	}

	$: elem = ingredient.modifiers.includes('RECIPE') ? ('a' as const) : ('span' as const);
</script>

<svelte:element
	this={elem}
	href={elem === 'a' ? `/recipe?${new URLSearchParams({ r: ingredient.name })}` : null}
	class:underline-link={elem === 'a'}
	use:quantityHighlight={{ index }}
	data-highlight-cls="qhighlight">{displayName(ingredient)}</svelte:element
>{#if subscript !== null}
	<sub>{subscript}</sub>
{/if}{#if ingredient.modifiers.includes('OPT')}
	{' '}(opt)
{/if}{#if ingredient.relation.type === 'reference' && ingredient.relation.reference_target !== 'ingredient'}
	{#if ingredient.relation.reference_target === 'step'}
		<span class="text-base-11">
			{' '} from
			<a
				href={`#step-${$sectionIndex}-${ingredient.relation.references_to}`}
				class="underline font-italic"
				use:intermediateIgrHighlight={{
					relation: ingredient.relation
				}}>{getStepName($steps[ingredient.relation.references_to])}</a
			></span
		>{:else}<span class="text-base-11">
			{' '} from
			<a
				href={`#section-${ingredient.relation.references_to}`}
				class="underline font-italic"
				use:intermediateIgrHighlight={{
					relation: ingredient.relation
				}}>a previous section</a
			></span
		>{/if}{/if}{#if ingredient.quantity}
	:
	<Quantity class="text-base-11" quantity={ingredient.quantity} />
{/if}
