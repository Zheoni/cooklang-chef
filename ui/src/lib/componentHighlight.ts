import type { ComponentRelation, IngredientRelation } from './types';

export function componentHighlight<
	T extends { relation: ComponentRelation & Partial<IngredientRelation> }
>(
	element: HTMLElement,
	params: {
		component: T;
		index: number;
		componentKind: 'ingredient' | 'cookware';
		currentSectionIndex?: number;
	}
) {
	let ref_group =
		params.component.relation.type === 'definition'
			? params.index.toString()
			: params.component.relation.references_to.toString();
	let componentKind = params.componentKind;
	if (
		componentKind === 'ingredient' &&
		params.component.relation.type === 'reference' &&
		params.component.relation.reference_target &&
		params.component.relation.reference_target !== 'ingredient'
	) {
		if (params.component.relation.reference_target === 'step') {
			componentKind += '-step';
			if (params.currentSectionIndex === undefined) {
				throw new Error('no currentSectionIndex');
			}
			ref_group += '-' + params.currentSectionIndex;
		} else {
			componentKind += '-section';
		}
	}
	element.setAttribute('data-component-ref-group', ref_group.toString());
	element.setAttribute('data-component-kind', componentKind);
	const selector = `[data-component-ref-group="${ref_group}"][data-component-kind="${componentKind}"]`;
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

export function intermediateIgrHighlight(
	element: HTMLElement,
	params: { relation: IngredientRelation }
) {
	if (params.relation.type === 'definition') {
		return;
	}

	if (params.relation.reference_target === 'section') {
		intermediateSection(element, params.relation.references_to);
	} else if (params.relation.reference_target === 'step') {
		intermediateStep(element, params.relation.references_to);
	}
}

function intermediateSection(element: HTMLElement, sectionIndex: number) {
	const sectionElement = document.querySelector(`[data-section-index="${sectionIndex}"]`);
	if (sectionElement === null) {
		return console.error('Could not find parent section for ingredient', element);
	}
	element.addEventListener('mouseenter', () => {
		sectionElement.classList.add(cls(sectionElement));
	});
	element.addEventListener('mouseleave', () => {
		sectionElement.classList.remove(cls(sectionElement));
	});
}
function intermediateStep(element: HTMLElement, stepIndex: number) {
	const sectionElement = element.closest(`[data-section-index]`);
	const sectionIndex = sectionElement && sectionElement.getAttribute('data-section-index');
	if (sectionElement === null || sectionIndex === null) {
		return console.error('Could not find parent section for ingredient', element);
	}
	const stepElement = sectionElement.querySelector(`[data-step-index="${stepIndex}"]`);
	if (stepElement === null) {
		return console.error('Could not find step element for', element);
	}
	element.addEventListener('mouseenter', () => {
		stepElement.classList.add(cls(stepElement));
	});
	element.addEventListener('mouseleave', () => {
		stepElement.classList.remove(cls(stepElement));
	});
}
