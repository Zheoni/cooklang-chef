<script lang="ts">
	import { locale } from '$lib/i18n';
	import En from './en.svelte';
</script>

{#if $locale === 'en'}
	{#await import('./en.svelte') then c}
		<svelte:component this={c.default} />
	{/await}
{:else if $locale === 'es'}
	{#await import('./es.svelte') then c}
		<svelte:component this={c.default} />
	{/await}
{:else if $locale === 'de'}
	{#await import('./de.svelte') then c}
		<svelte:component this={c.default} />
	{/await}
{:else}
	<En />
{/if}
