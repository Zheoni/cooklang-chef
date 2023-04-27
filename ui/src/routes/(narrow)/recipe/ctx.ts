import { persistent } from '$lib/persistent';
import { getContext, setContext } from 'svelte';

const KEY = 'recipeViewSettings';

export function initCtx() {
	const stepIngredientsView = persistent<'compact' | 'list' | 'hidden'>(
		'compact',
		'compactStepIngredients'
	);
	return setContext(KEY, { stepIngredientsView });
}

export function getCtx() {
	return getContext<ReturnType<typeof initCtx>>(KEY);
}
