<script lang="ts">
	export let variant: 'normal' | 'dashed' | 'dotted' = 'normal';
	export let labelPos: 'left' | 'center' | 'right' = 'center';

	$: className = $$props.class ?? '';
</script>

<div
	class="text-base-11 flex h-0 select-none items-center whitespace-nowrap {className}"
	class:border-dashed={variant === 'dashed'}
	class:border-dotted={variant === 'dotted'}
	class:left={labelPos === 'left'}
	class:right={labelPos === 'right'}
>
	<slot />
</div>

<style>
	div::after,
	div::before {
		content: '';
		border-style: inherit;
		--at-apply: h-0.5 flex-grow-1 border-t border-current text-base-7;
	}

	div:not(:empty)::before {
		--at-apply: me-2;
	}
	div:not(:empty)::after {
		--at-apply: ms-2;
	}

	.left::before {
		flex-grow: 0;
		--at-apply: min-w-8;
	}
	.right::after {
		flex-grow: 0;
		--at-apply: min-w-8;
	}
</style>
