import t from 'twemoji';

export default function twemoji(node: HTMLElement, _params?: { emoji?: string }) {
	const options = { ext: '.svg', folder: 'svg' };

	function apply() {
		t.parse(node, options);
	}
	apply();
	return {
		update(params?: { emoji?: string }) {
			if (params?.emoji) {
				node.innerHTML = t.parse(params.emoji, options);
			}
		}
	};
}
