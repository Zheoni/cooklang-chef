import { tooltip } from 'svooltip';
import type { Recipe, ScaleOutcome } from './types';
import { get } from 'svelte/store';
import { t } from './i18n';

export function scaleOutcomeTooltip(node: HTMLElement, outcome: ScaleOutcome | null) {
	let tt = tooltip(node, { visibility: false, content: 'empty', placement: 'bottom' });
	function apply(outcome: ScaleOutcome | null) {
		node.classList.remove('scale-error', 'scale-fixed');
		tt?.update({ visibility: false });
		if (outcome === 'Scaled') return;
		if (outcome === 'Error') {
			tt?.update({ content: get(t)('outcome.error'), placement: 'bottom', visibility: true });
			node.classList.add('scale-error');
		} else if (outcome === 'Fixed') {
			tt?.update({ content: get(t)('outcome.fixed'), placement: 'bottom', visibility: true });
			node.classList.add('scale-fixed');
		}
	}

	apply(outcome);

	return {
		update(outcome: ScaleOutcome | null) {
			apply(outcome);
		}
	};
}

export function extractOutcome(recipe: Recipe, index: number) {
	if (recipe.data.type === 'DefaultScaling') return null;
	return recipe.data.ingredients[index];
}
