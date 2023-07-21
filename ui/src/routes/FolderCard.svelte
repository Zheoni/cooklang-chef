<script lang="ts">
	import Folder from '~icons/lucide/folder';

	export let dir: string;

	$: name = dir.split('/').reverse()[0];
	$: href = `/?${new URLSearchParams({ dir })}`;
</script>

<a
	{href}
	class="block bg-base-3 hover:bg-base-4 hover:border-primary-9 transition-border-color h-20 md:min-w-40 md:h-30 rounded-xl border-2 border-transparent shadow-xl"
>
	<div class="flex md:flex-col items-center justify-center h-full p-2">
		<Folder class="text-2xl text-primary-9" />
		<div>
			<h2 class="font-heading text-xl px-4">{name}</h2>
		</div>
	</div>
</a>
