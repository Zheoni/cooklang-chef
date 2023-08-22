<svelte:options immutable={true} />

<script lang="ts">
	import { getContext } from 'svelte';
	import Quantity from './Quantity.svelte';
	import { quantityHighlight, intermediateIgrHighlight } from './componentHighlight';
	import type { Ingredient, Step } from './types';
	import { displayName } from './util';
	import type { SectionContext } from './Section.svelte';
	import { t } from './i18n';

	export let index: number;
	export let ingredient: Ingredient;
	export let subscript: number | null = null;

	const { sectionIndex, steps } = getContext<SectionContext>('section');

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
		{@const step = $steps[ingredient.relation.references_to]}
		<span class="text-base-11">
			{' '}
			<a
				href={`#step-${$sectionIndex}-${ingredient.relation.references_to}`}
				class="underline font-italic"
				use:intermediateIgrHighlight={{
					relation: ingredient.relation
				}}>{step.number ? $t('r.ref.fromStep', { step: step.number }) : $t('r.ref.fromPrevStep')}</a
			></span
		>{:else}<span class="text-base-11">
			{' '}
			<a
				href={`#section-${ingredient.relation.references_to}`}
				class="underline font-italic"
				use:intermediateIgrHighlight={{
					relation: ingredient.relation
				}}>{$t('r.ref.fromPrevSect')}</a
			></span
		>{/if}{/if}{#if ingredient.quantity}
	:
	<Quantity class="text-base-11" quantity={ingredient.quantity} />
{/if}
