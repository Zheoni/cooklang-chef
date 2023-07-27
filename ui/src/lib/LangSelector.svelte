<script lang="ts">
	import Listbox from './listbox/Listbox.svelte';
	import { allLocales, locale, localesData } from './i18n';
	import { ListboxButton, ListboxLabel } from '@rgossiaux/svelte-headlessui';
	import twemoji from './twemoji';
	import ListboxOptions from './listbox/ListboxOptions.svelte';
	import ListboxOption from './listbox/ListboxOption.svelte';

	// TODO: update URL to new locale
</script>

<Listbox bind:value={$locale} placement="top">
	<svelte:fragment slot="button">
		<ListboxButton class="bg-base-3 hover:bg-base-4 rounded px-3 text-base-11">
			<span use:twemoji={{ emoji: $localesData[$locale].emoji }}>{$localesData[$locale].emoji}</span
			>
			{$localesData[$locale].name}
		</ListboxButton>
	</svelte:fragment>
	<svelte:fragment slot="label">
		<!-- ? should this be localised -->
		<ListboxLabel class="sr-only">Language</ListboxLabel>
	</svelte:fragment>
	<ListboxOptions>
		{#each $allLocales as loc}
			{@const d = $localesData[loc]}
			<ListboxOption value={loc}>
				<span use:twemoji class="pe-2">{d.emoji}</span>
				{d.name}
			</ListboxOption>
		{/each}
	</ListboxOptions>
</Listbox>
