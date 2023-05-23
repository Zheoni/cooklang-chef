<script lang="ts">
	import { scale } from 'svelte/transition';

	import { offset, flip, shift, type Placement } from 'svelte-floating-ui/dom';
	import { createFloatingActions } from 'svelte-floating-ui';
	import { Listbox } from '@rgossiaux/svelte-headlessui';

	export let value: string | number;
	export let placement: Placement = 'bottom-end';

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		placement,
		middleware: [offset({ mainAxis: 6, crossAxis: 6 }), flip(), shift({ crossAxis: true })]
	});
</script>

<Listbox bind:value let:open on:change class="font-sans text-base">
	<div use:floatingRef>
		<slot name="button" />
	</div>
	<slot name="label" />
	{#if open}
		<div
			transition:scale={{ duration: 150, start: 0.9 }}
			use:floatingContent
			class="absolute origin-top-right"
		>
			<slot {open} />
		</div>
	{/if}
</Listbox>
