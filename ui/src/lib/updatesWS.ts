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

const _connected = writable(false); // as it will connect instantly, false will be set in case of error
export const connected = readonly(_connected);

let ws: WebSocket | null = null;

export function connect(trusted = false) {
	if (!browser) throw new Error('Tried to connect to the updates web socket not in the browser');

	const path = `${API}/updates`;
	const currentUrl = get(page).url;
	const base = `${currentUrl.protocol.replace('http', 'ws')}//${currentUrl.host}`;
	const url = new URL(path, base);

	ws = new WebSocket(url);
	ws.addEventListener('open', () => _connected.set(true));
	ws.addEventListener('message', (message) => {
		const data = JSON.parse(message.data) as Message;
		console.log(data);
		let name = data.path.replace('.cook', '').replace('\\', '/');
		name = encodeURI(name);
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
		invalidate((url) => url.pathname.endsWith(name));
	});
	ws.addEventListener('close', () => {
		if (trusted) toast.error('Could not set up auto update');
		_connected.set(false);
	});
	return () => {
		if (ws) ws.close();
	};
}
