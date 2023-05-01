<div
	class="border-base-6 my-2 flex w-fit flex-wrap items-center justify-start gap-2 rounded border border-dotted p-2"
>
	<div
		class="inset-shadow radix-solid-primary flex h-9 w-9 items-center justify-center rounded-full"
	>
		<slot name="icon" />
	</div>
	<slot />
</div>

<style>
	.inset-shadow {
		box-shadow: #0000001a 0 0 0 1px inset, #0000002f 0 0 4px 3px inset;
	}
</style>
