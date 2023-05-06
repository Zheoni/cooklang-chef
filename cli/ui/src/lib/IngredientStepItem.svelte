<script lang="ts">
	import Quantity from './Quantity.svelte';
	import { quantityHighlight } from './componentHighlight';
	import type { Ingredient } from './types';
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
	use:quantityHighlight={{ index }}
	data-highlight-cls="qhighlight">{displayName(ingredient)}</svelte:element
>{#if subscript !== null}
	<sub>{subscript}</sub>
{/if}{#if ingredient.modifiers.includes('OPT')}
	{' '}(opt)
{/if}{#if ingredient.quantity}
	:
	<Quantity class="text-base-11" quantity={ingredient.quantity} />
{/if}
