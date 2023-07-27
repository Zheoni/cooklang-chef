<script lang="ts">
	import { page } from '$app/stores';
	import toast from '$lib/toast';
	import { API } from './constants';
	import Code from '~icons/lucide/code-2';
	import { t } from '$lib/i18n';

	async function openEditor(srcPath: string) {
		const response = await fetch(`${API}/recipe/open_editor/${srcPath}`);
		if (!response.ok) {
			toast.error($t('openInEditor.error'));
			console.error('Could not open editor:', response.status, response.statusText);
			return;
		}
		toast.success($t('openInEditor.success'));
	}

	$: isLoopback =
		$page.url.hostname === 'localhost' ||
		$page.url.hostname === '127.0.0.1' ||
		$page.url.hostname === '[::1]';

	export let srcPath: string;
</script>

{#if isLoopback}
	<button
		class="btn radix-solid-primary px-2 py-1 gap-1 flex! items-center"
		on:click={() => openEditor(srcPath)}
	>
		<Code />
		{$t('openInEditor.text')}
	</button>
{/if}
