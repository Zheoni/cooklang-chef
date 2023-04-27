import type { Ingredient } from './types';

export function ingredientHighlight(
	element: HTMLElement,
	params: {
		ingredient: Ingredient;
		index: number;
	}
) {
	const ref_group = params.ingredient.references_to || params.index;
	element.setAttribute('data-ingredient-ref-group', ref_group.toString());
	const selector = `[data-ingredient-ref-group="${ref_group}"`;
	element.addEventListener('mouseenter', () => {
		document.querySelectorAll(selector).forEach((el) => el.classList.add(cls(el)));
	});
	element.addEventListener('mouseleave', () => {
		document.querySelectorAll(selector).forEach((el) => el.classList.remove(cls(el)));
	});
}

export function quantityHighlight(
	element: HTMLElement,
	params: {
		index: number;
	}
) {
	element.setAttribute('data-quantity-ref', params.index.toString());
	const selector = `[data-quantity-ref="${params.index}"]`;
	element.addEventListener('mouseenter', () => {
		document.querySelectorAll(selector).forEach((el) => el.classList.add(cls(el)));
	});
	element.addEventListener('mouseleave', () => {
		document.querySelectorAll(selector).forEach((el) => el.classList.remove(cls(el)));
	});
}

function cls(el: Element) {
	return el.getAttribute('data-highlight-cls') || 'highlight';
}
