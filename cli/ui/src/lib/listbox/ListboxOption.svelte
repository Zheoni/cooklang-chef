<script lang="ts">
	import { ListboxOption } from '@rgossiaux/svelte-headlessui';

	export let value: string | number;
	export let unstyled = false;
</script>

<ListboxOption {value} let:active let:selected let:disabled>
	{#if unstyled}
		<slot {active} {selected} {disabled} />
	{:else}
		<div
			class="my-1 flex h-12 w-full flex-grow items-center border-l-4 rounded font-semibold px-2 overflow-hidden cursor-pointer"
			class:border-primary-9={selected}
			class:border-transparent={!selected}
			class:bg-base-4={active}
		>
			<slot {active} {selected} {disabled} />
		</div>
	{/if}
</ListboxOption>
