<script lang="ts" context="module">
	import type { Quantity, QuantityValue, Value } from './types';

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

	export function qValueFmt(val: QuantityValue) {
		switch (val.type) {
			case 'fixed':
			case 'linear':
				return valueToString(val.value);
			case 'byServings':
				return valueToString(val.value[0]);
		}
	}
</script>

<script lang="ts">
	export let quantity: Quantity;

	$: value = qValueFmt(quantity.value);
</script>

<span>
	{value}
	{#if quantity.unit}
		<span class="font-italic">{quantity.unit}</span>
	{/if}
</span>
