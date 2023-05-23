<script lang="ts">
	import Quantity from './Quantity.svelte';
	import { componentHighlight } from './componentHighlight';
	import type { Ingredient, Quantity as Q } from './types';
	import ChefHat from '~icons/lucide/chef-hat';
	import { displayName } from './util';

	export let index: number;
	export let ingredient: Ingredient;
	export let quantities: Q[];

	$: elem = ingredient.modifiers.includes('RECIPE') ? ('a' as const) : ('span' as const);
</script>

{#if !ingredient.modifiers.includes('HIDDEN')}
	<svelte:element
		this={elem}
		class:underline-link={elem === 'a'}
		href={elem === 'a' ? `/recipe?${new URLSearchParams({ r: ingredient.name })}` : null}
		class="inline-block first-letter:uppercase"
		use:componentHighlight={{ index, component: ingredient, componentKind: 'ingredient' }}
		>{displayName(ingredient)}{#if elem === 'a'}
			<ChefHat />
		{/if}</svelte:element
	>{#if ingredient.modifiers.includes('OPT')}
		{' '}(opt)
	{/if}{#if quantities.length > 0}
		:
		{#each quantities.slice(0, quantities.length - 1) as q}
			<Quantity class="text-base-11" quantity={q} />{', '}
		{/each}
		<Quantity class="text-base-11" quantity={quantities[quantities.length - 1]} />
	{/if}
	{#if ingredient.note}
		<div class="ms-2 text-sm bg-yellow-3 p-2 rounded border border-yellow-6">
			{ingredient.note}
		</div>
	{/if}
{/if}
