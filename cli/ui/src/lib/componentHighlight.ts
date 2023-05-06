import type { ComponentRelation } from './types';

export function componentHighlight<T extends { relation: ComponentRelation }>(
	element: HTMLElement,
	params: {
		component: T;
		index: number;
		componentKind: 'ingredient' | 'cookware';
	}
) {
	const ref_group =
		params.component.relation.type === 'definition'
			? params.index
			: params.component.relation.references_to;
	element.setAttribute('data-component-ref-group', ref_group.toString());
	element.setAttribute('data-component-kind', params.componentKind);
	const selector = `[data-component-ref-group="${ref_group}"][data-component-kind="${params.componentKind}"]`;
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
	return el.getAttribute('data-highlight-cls') ?? 'highlight';
}
