<script lang="ts">
	import Folder from '~icons/lucide/folder';
	import Card from './Card.svelte';

	export let dir: string;

	$: name = dir.split('/').reverse()[0];
</script>

<Card as="a" href={`/?${new URLSearchParams({ dir })}`}>
	<div class="flex flex-col items-center justify-center h-full">
		<Folder class="text-6xl text-primary-9" />
		<div>
			<h2 class="font-heading text-2xl">{name}</h2>
		</div>
	</div>
</Card>
