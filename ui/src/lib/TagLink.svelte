<script lang="ts">
	import Tag from './Tag.svelte';

	export let text: string;
</script>

<a href={`/?${new URLSearchParams({ tag: text })}`} data-sveltekit-preload-data="tap">
	<Tag {text} link={false} />
</a>
