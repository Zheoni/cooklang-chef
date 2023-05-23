<script lang="ts">
	import Loader2 from '~icons/lucide/loader-2';
</script>

<div>
	<p class="sr-only">Loading</p>
	<Loader2 class="animate-spin" />
</div>

<style>
	div {
		animation-name: fade;
		animation-delay: 250;
		animation-duration: 200ms;
		animation-fill-mode: forwards;
		opacity: 0;
	}
	@keyframes fade {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
</style>
