<script lang="ts">
	import Tag from './Tag.svelte';

	export let text: string;
	export let color: string | null = null;
	export let emoji: string | null = null;
</script>

<a href={`/?${new URLSearchParams({ tag: text })}`} data-sveltekit-preload-data="tap">
	<Tag {text} {color} {emoji} link={false} />
</a>
