<script lang="ts">
	import { page } from '$app/stores';
	import DisplayReport from '$lib/DisplayReport.svelte';
	$: error = $page.error!;
	$: errors = (error.parse_errors ?? []).concat(error.parse_warnings ?? []);
</script>

<h1 class="text-2xl text-red-9">{$page.error?.message ?? 'Unknown error'}</h1>
{#if error.code === 'PARSE'}
	<DisplayReport ansiString={error.fancy_report ?? null} {errors} srcPath={error.srcPath ?? null} />
{/if}
