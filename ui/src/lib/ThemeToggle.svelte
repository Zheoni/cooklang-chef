<script lang="ts">
	import Listbox from './listbox/Listbox.svelte';
	import {
		colorScheme,
		adjustedColorScheme,
		systemColorScheme,
		resolveAdjustedColors,
		type AdjustedColorScheme
	} from './colorScheme';
	import Sun from '~icons/lucide/sun';
	import Moon from '~icons/lucide/moon';
	import DeviceDesktop from '~icons/lucide/monitor';
	import { ListboxButton, ListboxLabel } from '@rgossiaux/svelte-headlessui';
	import ListboxOption from './listbox/ListboxOption.svelte';
	import ListboxOptions from './listbox/ListboxOptions.svelte';
	import { tick } from 'svelte';
	import { get } from 'svelte/store';

	const themes = [
		{ id: 'light', name: 'Light', icon: Sun },
		{ id: 'dark', name: 'Dark', icon: Moon },
		{ id: 'system', name: 'System', icon: DeviceDesktop }
	] as const;

	// @ts-expect-error: Transition API
	// prettier-ignore
	const isAppearanceTransition = document.startViewTransition
		&& !window.matchMedia('(prefers-reduced-motion: reduce)').matches;

	async function changeTheme(adjusted: AdjustedColorScheme) {
		const cs = resolveAdjustedColors(adjusted, $systemColorScheme);
		if (!isAppearanceTransition || cs == $colorScheme) {
			adjustedColorScheme.set(adjusted);
			return;
		}

		// Wait for menu to close
		await new Promise((r) => setTimeout(r, 150));

		// Position of the theme toggle button
		const btn = document.getElementById('theme-toggle')?.getBoundingClientRect();
		let x: number;
		let y: number;
		if (btn) {
			x = btn.x + btn.width / 2;
			y = btn.y + btn.height / 2;
		} else {
			x = innerWidth / 2;
			y = innerHeight / 2;
		}

		// Get the distance to the furthest corner
		const endRadius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));

		// Create a transition:
		// @ts-expect-error: Transition API
		const transition = document.startViewTransition(async () => {
			adjustedColorScheme.set(adjusted);
			await tick();
		});

		// Wait for the pseudo-elements to be created:
		transition.ready.then(() => {
			const clipPathNormal = [
				`circle(0px at ${x}px ${y}px)`,
				`circle(${endRadius}px at ${x}px ${y}px)`
			];
			const clipPath = cs === 'dark' ? [...clipPathNormal].reverse() : clipPathNormal;
			const element = cs === 'dark' ? '::view-transition-old(root)' : '::view-transition-new(root)';
			const easing = cs === 'dark' ? 'ease-out' : 'ease-in';

			// Animate the root’s new view
			document.documentElement.animate(
				{
					clipPath
				},
				{
					duration: 400,
					easing,
					// Specify which pseudo-element to animate
					pseudoElement: element
				}
			);
		});
	}

	let listboxTheme = get(adjustedColorScheme);
	$: changeTheme(listboxTheme);
</script>

<Listbox bind:value={listboxTheme}>
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
					class="my-1 flex h-12 w-full items-center rounded border-l-4 font-semibold"
					class:border-transparent={!selected}
					class:active-dark={selected && $colorScheme === 'dark'}
					class:active-light={selected && $colorScheme === 'light'}
					class:active
				>
					<span class="px-2 pt-1"><svelte:component this={theme.icon} /></span>
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
