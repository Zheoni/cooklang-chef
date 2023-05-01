import t from 'twemoji';

export default function twemoji(node: HTMLElement) {
	t.parse(node, { ext: '.svg', folder: 'svg' });
}
