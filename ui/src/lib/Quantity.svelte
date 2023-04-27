<script lang="ts" context="module">
	import type { Quantity, QuantityValue, Value } from './types';

	export function valueToString(val: Value) {
		switch (val.type) {
			case 'number':
				return val.value.toString();
			case 'range':
				return `${val.value.start}-${val.value.end}`;
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
