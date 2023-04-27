import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';
import UnoCSS from 'unocss/vite';

export default defineConfig({
	plugins: [
		UnoCSS({
			mode: 'svelte-scoped'
		}),
		Icons({ compiler: 'svelte', scale: 1.2, defaultClass: 'icon', autoInstall: true }),
		sveltekit()
	],
	server: {
		proxy: {
			'/api': {
				target: 'http://127.0.0.1:8080',
				ws: true
			}
		}
	}
});
