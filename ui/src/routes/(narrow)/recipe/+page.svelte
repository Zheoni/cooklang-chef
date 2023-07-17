<script lang="ts">
	import Tag from '$lib/Tag.svelte';
	import twemoji from '$lib/twemoji';
	import type { PageData, Snapshot } from './$types';
	import Utensils from '~icons/lucide/utensils';
	import User from '~icons/lucide/user';
	import Hourglass from '~icons/lucide/hourglass';
	import Calendar from '~icons/lucide/calendar';
	import External from '~icons/lucide/external-link';
	import Code from '~icons/lucide/code-2';
	import FileCode from '~icons/lucide/file-code';
	import Scale from '~icons/lucide/scale';
	import Ruler from '~icons/lucide/ruler';

	import StepIngredientsViewIcon from '~icons/lucide/align-vertical-distribute-center';
	import Metadata from '$lib/Metadata.svelte';
	import MetadataGroup from '$lib/MetadataGroup.svelte';
	import DisplayReport from '$lib/DisplayReport.svelte';
	import Section from './Section.svelte';
	import { API } from '$lib/constants';
	import Listbox from '$lib/listbox/Listbox.svelte';
	import {
		ListboxButton,
		ListboxLabel,
		Popover,
		PopoverButton,
		PopoverPanel
	} from '@rgossiaux/svelte-headlessui';
	import ListboxOptions from '$lib/listbox/ListboxOptions.svelte';
	import ListboxOption from '$lib/listbox/ListboxOption.svelte';
	import { qValueFmt } from '$lib/Quantity.svelte';
	import { componentHighlight } from '$lib/componentHighlight';
	import { stepIngredientsView } from '$lib/settings';
	import OpenInEditor from '$lib/OpenInEditor.svelte';
	import { connected } from '$lib/updatesWS';
	import { fade, scale } from 'svelte/transition';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { createFloatingActions } from 'svelte-floating-ui';
	import { flip, offset, shift } from 'svelte-floating-ui/dom';
	import Divider from '$lib/Divider.svelte';
	import { scaleOutcomeTooltip } from '$lib/scaleOutcomeTooltip';
	import IngredientListItem from '$lib/IngredientListItem.svelte';
	import { displayName, formatTime } from '$lib/util';
	import VideoEmbed from '$lib/VideoEmbed.svelte';
	import TimerClock from '$lib/TimerClock.svelte';
	import { onMount } from 'svelte';

	export let data: PageData;

	function fromEpochFormat(secsFromEpoch: number) {
		const date = new Date(0);
		date.setUTCSeconds(secsFromEpoch);
		return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
			date
		);
	}

	let state = {
		moreDataOpen: false,
		warningsOpen: false
	};
	export const snapshot: Snapshot<typeof state> = {
		capture: () => state,
		restore: (value) => (state = value)
	};

	$: ({ recipe, images, srcPath, warnings, fancy_report, created, modified } = data);

	$: defaultServings = (recipe.metadata.servings && recipe.metadata.servings[0]) ?? 1;
	$: urlServings = $page.url.searchParams.get('scale');
	$: selectedServings = (urlServings !== null && Number(urlServings)) || defaultServings;

	$: urlUnits = $page.url.searchParams.get('units');
	let selectedUnits: Options['units'];
	$: {
		if (urlUnits === 'metric' || urlUnits === 'imperial') {
			selectedUnits = urlUnits;
		} else {
			selectedUnits = 'default';
		}
	}

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		placement: 'bottom',
		middleware: [offset(6), flip(), shift({ crossAxis: true })]
	});

	type Options = {
		servings: number;
		units: 'metric' | 'imperial' | 'default';
	};

	let lastTimeout: number | null = null;
	function navToOptions(options: Options, delay: boolean) {
		if (lastTimeout) {
			clearTimeout(lastTimeout);
		}
		lastTimeout = setTimeout(
			() => {
				const params = new URLSearchParams($page.url.searchParams);
				if (options.servings === defaultServings) {
					params.delete('scale');
				} else {
					params.set('scale', options.servings.toString());
				}
				if (options.units !== 'default') {
					params.set('units', options.units);
				} else {
					params.delete('units');
				}
				if ($page.url.searchParams.toString() !== params.toString()) {
					goto(`?${params}`, { keepFocus: true, noScroll: true });
				}
			},
			delay ? 500 : 0
		);
	}

	let scalePopover: HTMLDivElement | null;
	$: navToOptions(
		{ servings: selectedServings, units: selectedUnits },
		scalePopover?.isConnected ?? false
	);
</script>

<svelte:head>
	<title>{recipe.name}</title>
</svelte:head>

{#if images.length > 0}
	<div class="w-full max-h-70vh overflow-hidden rounded shadow-lg mb-8">
		<!-- svelte-ignore a11y-missing-attribute -->
		<img class="object-cover h-full w-full" src={`${API}/src/${images[0].path}`} />
	</div>
{/if}

{#if warnings.length > 0}
	<details bind:open={state.warningsOpen}>
		<summary class="text-yellow-11 font-bold">Warnings</summary>

		<DisplayReport ansiString={fancy_report} errors={warnings} {srcPath} kind="warning" />
	</details>
{/if}
{#if $connected === 'connected'}
	<div class="float-right flex flex-wrap gap-2" transition:fade|local>
		<OpenInEditor {srcPath} />
	</div>
{/if}
<h1 class="text-6xl font-heading">
	{#if recipe.metadata.emoji}
		<span use:twemoji={{ emoji: recipe.metadata.emoji }} class="text-6xl" aria-hidden>
			{recipe.metadata.emoji}
		</span>
	{/if}
	{recipe.name}
</h1>
<div class="m-4 flex flex-wrap gap-2">
	{#each recipe.metadata.tags as tag}
		<Tag text={tag} />
	{/each}
</div>
{#if recipe.metadata.description}
	<p class="border-primary-9 bg-base-2 m-4 w-fit rounded border-l-4 p-4 text-xl shadow">
		{recipe.metadata.description}
	</p>
{/if}

{#if recipe.metadata.servings}
	<MetadataGroup>
		<Utensils slot="icon" />
		<Metadata key="Servings">
			<div class="flex divide-x divide-base-6">
				{#each recipe.metadata.servings as serving}
					{@const selected = serving === selectedServings}
					<div class="px-1">
						<button
							class="px-2 h-fit rounded border-2 decoration-2"
							class:border-primary-7={selected}
							class:border-transparent={!selected}
							on:click={() => (selectedServings = serving)}>{serving}</button
						>
					</div>
				{/each}
				{#if !recipe.metadata.servings.includes(selectedServings)}
					<div class="px-1">
						<i class="i-lucide-arrow-right text-primary-11" />
						<div class="px-2 h-fit rounded border border-dashed border-primary-7 inline-block">
							{selectedServings}
						</div>
					</div>
				{/if}
			</div>
		</Metadata>
	</MetadataGroup>
{/if}
{#if recipe.metadata.author || recipe.metadata.source}
	<MetadataGroup>
		<User slot="icon" />
		{#if recipe.metadata.author}
			{@const author = recipe.metadata.author}
			<Metadata key="Author">
				{#if author.name !== null && author.url === null}
					{recipe.metadata.author.name ?? ''}
				{:else if author.name === null && author.url !== null}
					<a href={author.url} class="link italic">author website<External /></a>
				{:else}
					<a href={author.url} class="link">{author.name}<External /></a>
				{/if}
			</Metadata>
		{/if}
		{#if recipe.metadata.source}
			{@const source = recipe.metadata.source}
			<Metadata key="Source">
				{#if source.name !== null && source.url === null}
					{recipe.metadata.source.name ?? ''}
				{:else if source.name === null && source.url !== null}
					<a href={source.url} class="link italic">source website<External /></a>
				{:else}
					<a href={source.url} class="link">{source.name}<External /></a>
				{/if}
			</Metadata>
		{/if}
	</MetadataGroup>
{/if}
{#if recipe.metadata.time}
	{@const time = recipe.metadata.time}
	<MetadataGroup>
		<Hourglass slot="icon" />
		{#if typeof time === 'number'}
			<Metadata key="Total time">
				{formatTime(time)}
			</Metadata>
		{:else}
			{#if time.prep_time && time.cook_time}
				<Metadata key="Total time">
					{formatTime(time.cook_time + time.prep_time)}
				</Metadata>
			{/if}
			{#if time.prep_time}
				<Metadata key="Prep time">
					{formatTime(time.prep_time)}
				</Metadata>
			{/if}
			{#if time.cook_time}
				<Metadata key="Cook time">
					{formatTime(time.cook_time)}
				</Metadata>
			{/if}
		{/if}
	</MetadataGroup>
{/if}

<details bind:open={state.moreDataOpen}>
	<summary class="w-fit">More data</summary>

	{#if created || modified}
		<MetadataGroup>
			<Calendar slot="icon" />
			{#if created}
				<Metadata key="Added">
					{fromEpochFormat(created)}
				</Metadata>
			{/if}
			{#if modified}
				<Metadata key="Modified">
					{fromEpochFormat(modified)}
				</Metadata>
			{/if}
		</MetadataGroup>
	{/if}

	<MetadataGroup>
		<Code slot="icon" />
		<Metadata key="Source file">
			<span class="bg-base-1 dark -my-1 rounded px-4 py-1 font-mono text-base-12">
				{srcPath}
			</span>
			<a
				href={`${API}/src/${srcPath}`}
				class="btn-square-8 -my-1 radix-solid-primary ms-4"
				target="_blank"><FileCode /></a
			>
		</Metadata>
	</MetadataGroup>
</details>

<VideoEmbed youtubeUrl={recipe.metadata.source?.url} />

<div class="flex justify-end items-center gap-2">
	<Listbox bind:value={selectedUnits}>
		<svelte:fragment slot="button">
			<ListboxButton class="btn-square-9 radix-solid-primary">
				<Ruler />
			</ListboxButton>
		</svelte:fragment>
		<svelte:fragment slot="label">
			<ListboxLabel class="sr-only">Convert recipe units</ListboxLabel>
		</svelte:fragment>
		<ListboxOptions>
			{#each ['default', 'metric', 'imperial'] as system}
				<ListboxOption value={system}>
					<span class="capitalize">{system}</span>
				</ListboxOption>
			{/each}
		</ListboxOptions>
	</Listbox>

	<Popover let:open class="flex items-center">
		<PopoverButton class="btn-square-9 radix-solid-primary" use={[floatingRef]}>
			<Scale /><span class="sr-only">Scale</span>
		</PopoverButton>

		{#if open}
			<div
				transition:scale={{ duration: 150, start: 0.9 }}
				class="absolute origin-top"
				bind:this={scalePopover}
			>
				<PopoverPanel
					static
					use={[floatingContent]}
					class="bg-base-2 border border-base-7 w-fit min-w-52 rounded-xl p-1 shadow z-100"
				>
					<div class="flex items-center justify-center gap-6 flex-nowrap mt-4 mb-2">
						<button
							class="i-lucide-minus p-2 -m-2"
							on:click={() => (selectedServings = Math.max(selectedServings - 1, 1))}
							class:text-primary-11={selectedServings > 1}
							disabled={selectedServings <= 1}
						/>
						<input
							type="number"
							bind:value={selectedServings}
							min="0"
							max="99"
							class="dark px-4 py-2 bg-base-3 text-primary-11 border border-base-6 rounded text-2xl font-bold w-15 text-center tabular-nums"
						/>
						<button
							class="i-lucide-plus p-2 -m-2"
							on:click={() => (selectedServings = Math.min(selectedServings + 1, 99))}
							class:text-primary-11={selectedServings < 99}
							disabled={selectedServings >= 99}
						/>
					</div>

					<div class="flex flex-row flex-wrap gap-2 justify-center my-2">
						{#each [0.5, 2, 4] as mul}
							{#if selectedServings * mul >= 1}
								<button
									class="btn radix-solid-primary px-2 py-1"
									on:click={() => (selectedServings = Math.floor(selectedServings * mul))}
								>
									x{mul}
								</button>
							{/if}
						{/each}
					</div>

					{#if recipe.metadata.servings}
						<Divider />
						<div class="text-sm text-base-11 mt-2 ms-1">Presets</div>
						<div class="flex flex-row flex-wrap gap-2 justify-center">
							{#each recipe.metadata.servings as serving}
								<button
									class="btn radix-solid-primary px-2 py-1"
									on:click={() => (selectedServings = serving)}
								>
									{serving}
								</button>
							{/each}
						</div>
					{/if}

					<div class="flex mb-1 me-1 gap-2">
						<button
							class="ms-auto text-base-11 text-sm"
							on:click={() => (selectedServings = defaultServings)}>Reset</button
						>
					</div>
				</PopoverPanel>
			</div>
		{/if}
	</Popover>
</div>

<div class="font-serif text-lg content">
	<div class="md:grid md:grid-cols-2">
		{#if recipe.grouped_ingredients.length > 0}
			<div>
				<h2 class="text-2xl my-2 font-heading">Ingredients</h2>
				<ul class="list-disc ms-6">
					{#each recipe.grouped_ingredients as { index, outcome, quantity }}
						{@const ingredient = recipe.ingredients[index]}
						{#if !ingredient.modifiers.includes('HIDDEN') && !ingredient.modifiers.includes('REF_TO_STEP') && !ingredient.modifiers.includes('REF_TO_SECTION')}
							<li use:scaleOutcomeTooltip={outcome} class="w-fit">
								<IngredientListItem {index} {ingredient} quantities={quantity} />
							</li>
						{/if}
					{/each}
				</ul>
			</div>
		{/if}
		{#if recipe.cookware.length > 0}
			<div>
				<h2 class="text-2xl my-2 font-heading">Cookware</h2>
				<ul class="list-disc ms-6">
					{#each recipe.cookware as item, index}
						{#if !item.modifiers.includes('HIDDEN') && !item.modifiers.includes('REF')}
							<li>
								<span
									class="capitalize"
									use:componentHighlight={{ index, component: item, componentKind: 'cookware' }}
									>{displayName(item)}</span
								>{#if item.quantity}
									: <span class="text-base-11">{qValueFmt(item.quantity)}</span>
								{/if}
							</li>
						{/if}
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	{#if recipe.sections.length > 0}
		<div class="flex flex-wrap justify-between">
			<h2 class="text-2xl my-3 font-heading">Method</h2>
			<div class="flex items-center">
				<Listbox bind:value={$stepIngredientsView}>
					<svelte:fragment slot="button">
						<ListboxButton class="btn-square-9 radix-solid-primary">
							<StepIngredientsViewIcon />
						</ListboxButton>
					</svelte:fragment>
					<svelte:fragment slot="label">
						<ListboxLabel class="sr-only">Step ingredients view:</ListboxLabel>
					</svelte:fragment>
					<ListboxOptions>
						{#each ['compact', 'list', 'hidden'] as view}
							<ListboxOption value={view}>
								<span class="capitalize">{view}</span>
							</ListboxOption>
						{/each}
					</ListboxOptions>
				</Listbox>
			</div>
		</div>
		{#each recipe.sections as section, section_index}
			<Section {recipe} {images} {section} {section_index} />
			{#if recipe.sections.length > 1 && section_index < recipe.sections.length - 1}
				<Divider class="mt-8 mb-6" />
			{/if}
		{/each}
	{/if}
</div>

<TimerClock />

<style>
	input[type='number']::-webkit-inner-spin-button,
	input[type='number']::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
		outline: none !important;
	}
</style>
