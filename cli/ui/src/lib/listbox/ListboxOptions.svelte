<script lang="ts">
	import { ListboxOptions } from '@rgossiaux/svelte-headlessui';
</script>

<ListboxOptions
	static
	class="bg-base-2 border-base-7 divide-base-6 w-52 origin-top-right select-none divide-y rounded-xl border p-1 shadow z-100"
>
	<slot />
</ListboxOptions>
