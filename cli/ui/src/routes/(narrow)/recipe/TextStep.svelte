<script lang="ts">
	import { API } from '$lib/constants';
	import type { SliceStep } from './Section.svelte';

	export let step: SliceStep;
</script>

<div class="flex gap-2 items-center flex-col-reverse lg:flex-row my-2">
	<p class="grow indent-4">
		{#each step.step.items as item}
			{#if item.type === 'text'}
				{item.value}
			{/if}
		{/each}
	</p>
	{#if step.image}
		<div class="rounded overflow-hidden max-w-40%">
			<!-- svelte-ignore a11y-missing-attribute -->
			<img class="h-full w-full object-cover" src={`${API}/src/${step.image}`} />
		</div>
	{/if}
</div>
