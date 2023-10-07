<script lang="ts" context="module">
	import { tooltip } from 'svooltip';
	import type { NumOrFrac, Quantity, Value } from './types';
	import { getNumVal } from './util';
	import { t } from './i18n';

	const numf = Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
	function fmt(n: NumOrFrac): [string, number] {
		if (n.type === 'regular') {
			const val = getNumVal(n);
			return [numf.format(val), 0];
		} else {
			const parts = [];
			const { whole, num, den, err } = n.value;
			if (whole != 0) parts.push(numf.format(whole));
			if (num != 0) parts.push(formatFraction(num, den));
			if (parts.length === 0) parts.push('0');
			return [parts.join(' '), err];
		}
	}

	function formatFraction(num: number, den: number) {
		if (den === 0) {
			return 'Inf'; // ...
		}

		const repr = `${numf.format(num)}/${numf.format(den)}`;

		const fractions = {
			'1/2': '½',
			'1/3': '⅓',
			'2/3': '⅔',
			'1/4': '¼',
			'3/4': '¾',
			'1/5': '⅕',
			'2/5': '⅖',
			'3/5': '⅗',
			'4/5': '⅘',
			'1/6': '⅙',
			'5/6': '⅚',
			'1/7': '⅐',
			'1/8': '⅛',
			'3/8': '⅜',
			'5/8': '⅝',
			'7/8': '⅞',
			'1/9': '⅑',
			'1/10': '⅒'
		} as Record<string, string>;

		return fractions[repr] ?? repr;
	}

	export function valueToString(val: Value): [string, number] {
		switch (val.type) {
			case 'number':
				return fmt(val.value);
			case 'range':
				const [start, es] = fmt(val.value.start);
				const [end, ee] = fmt(val.value.end);
				const err = es + ee;
				return [`${start}-${end}`, err];
			case 'text':
				return [val.value, 0];
		}
	}
</script>

<script lang="ts">
	import { Popover, PopoverButton, PopoverPanel } from '@rgossiaux/svelte-headlessui';
	import { scale } from 'svelte/transition';
	import { createFloatingActions } from 'svelte-floating-ui';
	import { flip, offset, shift } from 'svelte-floating-ui/dom';
	import Loader from './Loader.svelte';
	import { API } from './constants';
	import toast from './toast';

	export let quantity: Quantity;
	export let editable = true;

	$: quantity2 = quantity;
	$: [value, error] = valueToString(quantity2.value);
	$: unit = quantity2.unit;
	$: theresError = Math.abs(error) > 0.001;
	$: errorTooltip = `${$t('quantity.roundError')}: ${error >= 0 ? '+' : ''}${numf.format(error)}`;
	$: showTooltip = editable && theresError;

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		placement: 'bottom',
		middleware: [offset(6), flip(), shift({ crossAxis: true })]
	});

	async function selectConversion() {
		const url = `${API}/select_conversion`;
		const body = {
			value: quantity.value,
			unit: quantity.unit
		};
		const resp = await fetch(url, {
			method: 'POST',
			body: JSON.stringify(body),
			headers: {
				'Content-Type': 'application/json'
			}
		});
		if (!resp.ok) {
			toast.error($t('quantity.cantConvert'));
		}
		const conversions = (await resp.json()) as Quantity[];
		return conversions;
	}
</script>

{#if editable}
	<Popover let:open as="span">
		<PopoverButton as="span" use={[floatingRef]}>
			<span
				{...$$restProps}
				use:tooltip={{ content: errorTooltip, visibility: showTooltip }}
				class:warn-err={error !== 0}
				class:editable
				class:active={open}
			>
				{value}
				{#if unit}
					<span class="font-italic">{unit}</span>
				{/if}
			</span>
		</PopoverButton>

		{#if open}
			<div transition:scale={{ duration: 150, start: 0.9 }} class="absolute">
				<PopoverPanel
					static
					use={[floatingContent]}
					class="bg-base-2 border border-base-7 min-w-52 rounded-xl p-1 shadow z-100"
					let:close
				>
					{#await selectConversion()}
						<div class="h-20 grid place-items-center">
							<Loader />
						</div>
					{:then conversions}
						<div
							class="grid grid-cols-[repeat(auto-fit,1fr)] md:grid-cols-[repeat(3,1fr)] gap-2 p-1"
						>
							{#each conversions as c}
								<button
									class="btn bg-base-3 hover:bg-base-4 px-2 py-1 md:w-15ch md:h-8ch"
									on:click={() => {
										quantity2 = c;
										close(null);
									}}
								>
									<svelte:self editable={false} quantity={c} />
								</button>
							{/each}
						</div>
						<div class="flex mb-1 me-1 gap-2">
							<button
								class="ms-auto text-base-11 text-sm font-sans"
								on:click={() => {
									quantity2 = quantity;
									close(null);
								}}
							>
								{$t('quantity.reset')}
							</button>
						</div>
					{/await}
				</PopoverPanel>
			</div>
		{/if}
	</Popover>
{:else}
	<span
		{...$$restProps}
		use:tooltip={{ content: errorTooltip, visibility: showTooltip }}
		class:warn-err={theresError}
	>
		{value}
		{#if unit}
			<span class="font-italic">{unit}</span>
		{/if}
	</span>
{/if}

<style>
	.warn-err {
		--at-apply: text-yellow-12;
	}

	.editable {
		--at-apply: hover:cursor-pointer p1 -m1 rounded
	}

	.editable:not(.active) {
		--at-apply: hover:bg-base-4
	}

	.active {
		--at-apply: bg-base-5
	}
</style>
