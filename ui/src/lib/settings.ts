import { persistent } from './persistent';

export const showFolders = persistent(true, 'showFolders');

export const stepIngredientsView = persistent<'compact' | 'list' | 'hidden'>(
	'compact',
	'compactStepIngredients'
);
