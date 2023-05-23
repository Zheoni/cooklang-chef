<script lang="ts">
	import Listbox from './listbox/Listbox.svelte';
	import { colorScheme, adjustedColorScheme } from './colorScheme';
	import Sun from '~icons/lucide/sun';
	import Moon from '~icons/lucide/moon';
	import DeviceDesktop from '~icons/lucide/monitor';
	import { ListboxButton, ListboxLabel } from '@rgossiaux/svelte-headlessui';
	import ListboxOption from './listbox/ListboxOption.svelte';
	import ListboxOptions from './listbox/ListboxOptions.svelte';

	const themes = [
		{ id: 'light', name: 'Light', icon: Sun },
		{ id: 'dark', name: 'Dark', icon: Moon },
		{ id: 'system', name: 'System', icon: DeviceDesktop }
	] as const;
</script>

<Listbox value={$adjustedColorScheme} on:change={(e) => adjustedColorScheme.set(e.detail)}>
	<svelte:fragment slot="button">
		<ListboxButton class={() => `theme-btn btn-${$adjustedColorScheme}`}>
			<svelte:component this={$colorScheme === 'light' ? Sun : Moon} />
		</ListboxButton>
	</svelte:fragment>
	<svelte:fragment slot="label">
		<ListboxLabel class="sr-only">Theme</ListboxLabel>
	</svelte:fragment>
	<ListboxOptions>
		{#each themes as theme}
			<ListboxOption value={theme.id} let:selected let:active unstyled>
				<button
					class="my-1 flex h-12 w-full flex-grow items-center rounded border-l-4 font-semibold"
					class:border-transparent={!selected}
					class:active-dark={selected && $colorScheme === 'dark'}
					class:active-light={selected && $colorScheme === 'light'}
					class:active
				>
					<span class="px-2"><svelte:component this={theme.icon} /></span>
					{theme.name}
				</button>
			</ListboxOption>
		{/each}
	</ListboxOptions>
</Listbox>

<style>
	.active {
		--at-apply: bg-base-4;
	}
	
	.active-light {
		--at-apply: border-primary-9 text-primary-11;
	}

	.active.active-light {
		--at-apply: bg-primary-3;
	}

	.active-dark {
		--at-apply: border-indigo-9 text-indigo-11;
	}

	.active.active-dark {
		--at-apply: bg-indigo-3;
	}

	:global(.theme-btn) {
		--at-apply: btn-square-9 m-1 text-xl;
	}

	:global(.btn-light) {
		--at-apply: radix-solid-primary;
	}

	:global(.btn-dark) {
		--at-apply: radix-solid-indigo;
	}

	:global(.btn-system) {
		--at-apply: text-green-11 hover:bg-green-4;
	}

	:global(.dark) :global(.btn-system) {
		--at-apply: text-indigo-11 hover:bg-indigo-4;
	}
</style>
