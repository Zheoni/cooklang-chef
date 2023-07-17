<script lang="ts">
	import '@unocss/reset/tailwind.css';
	import '../app.css';
	import 'svooltip/styles.css';

	import ChefHat from '~icons/lucide/chef-hat';
	import ThemeToggle from '$lib/ThemeToggle.svelte';
	import twemoji from '$lib/twemoji';
	import Divider from '$lib/Divider.svelte';

	import { SvelteToast } from '@zerodevx/svelte-toast';

	import { connect, connected } from '$lib/updatesWS';
	import { onMount } from 'svelte';
	import { scale } from 'svelte/transition';

	import { tooltip } from 'svooltip';

	onMount(() => connect());
</script>

<SvelteToast options={{ pausable: true }} />

<div class="bg-b flex min-h-screen flex-col">
	<header>
		<nav class="mx-auto flex max-w-screen-xl items-center px-4 py-2">
			<div class="flex-1">
				<a
					href="/"
					class="hover:bg-base-4 inline-flex h-12 flex-grow-0 items-center rounded px-4 text-xl font-bold"
					>chef</a
				>
			</div>
			{#if $connected !== 'pending'}
				<div
					class="h-4 w-4 rounded-full border-2 transition-colors inline-block mx-4"
					class:bg-green-9={$connected === 'connected'}
					class:border-green-6={$connected === 'connected'}
					class:bg-red-9={$connected === 'disconnected'}
					class:border-red-6={$connected === 'disconnected'}
					use:tooltip={{
						content:
							$connected === 'connected'
								? 'Auto updating content'
								: 'Auto update unavailable. Reload to retry.'
					}}
					transition:scale={{ delay: 500 }}
				/>
			{/if}
			<div id="theme-toggle">
				<ThemeToggle />
			</div>
		</nav>
		<Divider class="px-10 text-2xl" variant="dashed">
			<ChefHat />
		</Divider>
	</header>

	<main class="container mx-auto my-10 px-3 xl:max-w-screen-xl flex-grow">
		<slot />
	</main>

	<div class="mt-auto">
		<Divider class="my-10 px-10" variant="dashed">
			<footer class="flex px-3 items-stretch gap-2">
				<span use:twemoji>Cooked with ❤️</span>
				<div aria-hidden="true" class=" border-l border-base-6" />
				<a href="/about" class="link">About</a>
			</footer>
		</Divider>
	</div>
</div>

<style uno:preflights uno:safelist global></style>
