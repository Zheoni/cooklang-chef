<script lang="ts">
	export let key: string;
</script>

<div class="border-base-6 flex items-stretch justify-center overflow-hidden rounded border">
	<div class="bg-primary-3 border-primary-9 whitespace-nowrap border-r-2 px-4 py-2 font-semibold">
		{key}
	</div>
	<div class="bg-base-3 flex items-stretch justify-center px-4 py-2">
		<slot />
	</div>
</div>
