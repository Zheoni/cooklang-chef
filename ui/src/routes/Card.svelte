<script lang="ts">
	export let as: 'div' | 'article' | 'a' = 'div';
</script>

<svelte:element
	this={as}
	class="block bg-base-3 hover:bg-base-4 hover:border-primary-9 transition-border-color min-w-200px min-h-150px max-w-xl overflow-hidden rounded-xl border-2 border-transparent shadow-xl flex-grow md:flex-grow-0"
	{...$$restProps}
>
	<slot />
</svelte:element>

<style>
	* :global(h2) {
		background: transparent;
		background-position: 0 0;
		background-size: 600% 100%;
	}

	*:hover :global(h2) {
		background-image: linear-gradient(
			90deg,
			rgba(70, 167, 88, 1) 20%,
			rgba(48, 164, 108, 1) 40%,
			rgba(5, 162, 194, 1) 55%,
			rgba(18, 165, 148, 1) 70%,
			rgba(48, 164, 108, 1) 85%,
			rgba(112, 225, 200, 1) 90%,
			rgba(70, 167, 88, 1) 95%
		);
		--at-apply: bg-clip-text text-transparent;
	}

	@media (prefers-reduced-motion: no-preference) {
		:global(h2) {
			animation: move-bg 60s linear infinite;
		}
		@keyframes move-bg {
			to {
				background-position: 600% 0;
			}
		}
	}
</style>
