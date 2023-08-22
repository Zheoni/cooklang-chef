<script lang="ts" context="module">
	import type { Quantity, Value } from './types';

	const num = Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
	export function valueToString(val: Value) {
		switch (val.type) {
			case 'number':
				return num.format(val.value);
			case 'range':
				return `${num.format(val.value.start)}-${num.format(val.value.end)}`;
			case 'text':
				return val.value;
		}
	}
</script>

<script lang="ts">
	export let quantity: Quantity;

	$: value = valueToString(quantity.value);
</script>

<span {...$$restProps}>
	{value}
	{#if quantity.unit}
		<span class="font-italic">{quantity.unit}</span>
	{/if}
</span>
