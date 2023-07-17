import { API } from './constants';
import { browser } from '$app/environment';
import { goto, invalidate } from '$app/navigation';
import { page } from '$app/stores';
import { get, readonly, writable } from 'svelte/store';
import toast from './toast';

type Message =
	| {
			type: 'created' | 'deleted' | 'modified';
			path: string;
	  }
	| {
			type: 'renamed';
			path: string;
			to: string;
	  };

type Connected = 'connected' | 'pending' | 'disconnected';

const _connected = writable<Connected>('pending');
export const connected = readonly(_connected);

let ws: WebSocket | null = null;

export function connect() {
	if (!browser) throw new Error('Tried to connect to the updates web socket not in the browser');

	const path = `${API}/updates`;
	const currentUrl = get(page).url;
	const base = `${currentUrl.protocol.replace('http', 'ws')}//${currentUrl.host}`;
	const url = new URL(path, base);

	ws = new WebSocket(url);
	ws.addEventListener('open', () => _connected.set('connected'));
	ws.addEventListener('message', (message) => {
		const data = JSON.parse(message.data) as Message;
		console.log(data);
		let name = data.path.replace('.cook', '').replace('\\', '/');
		const isCurrentRecipe = get(page).url.searchParams.get('r') === name;
		if (data.type === 'deleted' && isCurrentRecipe) {
			toast.error('Recipe deleted');
			return goto('/');
		}
		if (data.type === 'renamed' && isCurrentRecipe) {
			toast.success('Renamed recipe');
			return goto('/recipe?' + new URLSearchParams({ r: data.to.replace('.cook', '') }));
		}
		invalidate(`${API}/recipe`);
		invalidate(`${API}/recipe/metadata`);
		invalidate((url) => decodeURI(url.pathname).endsWith(name));
	});
	ws.addEventListener('close', () => {
		_connected.set('disconnected');
	});
	return () => {
		if (ws) ws.close();
	};
}
