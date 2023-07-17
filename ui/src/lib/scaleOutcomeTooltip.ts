import { tooltip } from 'svooltip';
import type { Recipe, ScaleOutcome } from './types';

export function scaleOutcomeTooltip(node: HTMLElement, outcome: ScaleOutcome | null) {
	let tt = tooltip(node, { visibility: false, content: 'empty', placement: 'bottom' });
	function apply(outcome: ScaleOutcome | null) {
		node.classList.remove('scale-error', 'scale-fixed');
		tt?.update({ visibility: false });
		if (outcome === 'Scaled') return;
		if (outcome === 'Error') {
			tt?.update({ content: 'Error scaling', placement: 'bottom', visibility: true });
			node.classList.add('scale-error');
		} else if (outcome === 'Fixed') {
			tt?.update({ content: 'This value does not scale', placement: 'bottom', visibility: true });
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
