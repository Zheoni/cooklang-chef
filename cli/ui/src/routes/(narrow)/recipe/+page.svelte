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

	import StepIngredientsViewIcon from '~icons/lucide/align-vertical-distribute-center';
	import Metadata from '$lib/Metadata.svelte';
	import MetadataGroup from '$lib/MetadataGroup.svelte';
	import DisplayReport from '$lib/DisplayReport.svelte';
	import Section from './Section.svelte';
	import { API } from '$lib/constants';
	import Listbox from '$lib/listbox/Listbox.svelte';
	import { ListboxButton, ListboxLabel } from '@rgossiaux/svelte-headlessui';
	import ListboxOptions from '$lib/listbox/ListboxOptions.svelte';
	import ListboxOption from '$lib/listbox/ListboxOption.svelte';
	import type { GroupedQuantity } from '$lib/types';
	import Quantity, { qValueFmt } from '$lib/Quantity.svelte';
	import { ingredientHighlight } from '$lib/ingredientHighlight';
	import { stepIngredientsView } from '$lib/settings';
	import OpenInEditor from '$lib/OpenInEditor.svelte';
	import { connected } from '$lib/updatesWS';
	import { fade } from 'svelte/transition';

	export let data: PageData;

	function formatTime(minutes: number) {
		let hours = Math.trunc(minutes / 60);
		minutes %= 60;
		const days = Math.trunc(hours / 24);
		hours %= 24;

		const parts = [];
		// TODO maybe not construct formatters in every call
		if (days > 0) {
			parts.push(new Intl.NumberFormat(undefined, { style: 'unit', unit: 'day' }).format(days));
		}
		if (hours > 0) {
			parts.push(new Intl.NumberFormat(undefined, { style: 'unit', unit: 'hour' }).format(hours));
		}
		if (minutes > 0) {
			parts.push(
				new Intl.NumberFormat(undefined, { style: 'unit', unit: 'minute' }).format(minutes)
			);
		}
		return parts.join(' ');
	}

	function fromEpochFormat(secsFromEpoch: number) {
		const date = new Date(0);
		date.setUTCSeconds(secsFromEpoch);
		return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
			date
		);
	}

	function allQuantities(group: GroupedQuantity) {
		const all = [];
		for (const q of Object.values(group.known)) {
			if (q) all.push(q);
		}
		for (const q of Object.values(group.unknown)) {
			if (q) all.push(q);
		}
		if (group.no_unit) {
			all.push(group.no_unit);
		}
		for (const q of group.other) {
			all.push(q);
		}
		return all;
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
{#if $connected}
	<div class="float-right flex flex-wrap gap-2" transition:fade>
		<OpenInEditor {srcPath} />
	</div>
{/if}
<h1 class="text-6xl font-heading">
	{#if recipe.metadata.emoji}
		<span use:twemoji class="text-6xl" aria-hidden>
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
			{recipe.metadata.servings.join(', ')}
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
					<a href={author.url} class="link italic">author website</a>
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
					<a href={source.url} class="link italic">source website</a>
				{:else}
					<a href={source.url} class="link"
						>{source.name}<External class="inline-block -translate-y-1" /></a
					>
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
	<summary>More data</summary>

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
			<span class="bg-base-1 dark -my-1 rounded px-4 py-1 font-mono">
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

<div class="font-serif text-lg content">
	<div class="md:grid md:grid-cols-2">
		{#if recipe.ingredient_list.length > 0}
			<div>
				<h2 class="text-2xl my-2 font-heading">Ingredients</h2>
				<ul class="list-disc ms-6">
					{#each recipe.ingredient_list as entry}
						{@const ingredient = recipe.ingredients[entry.index]}
						{@const all = allQuantities(entry.quantity)}
						{#if !ingredient.modifiers.includes('HIDDEN')}
							<li>
								<span
									class="capitalize"
									use:ingredientHighlight={{ ingredient, index: entry.index }}
									>{ingredient.alias ?? ingredient.name}</span
								>{#if all.length > 0}
									:
									<span class="text-base-11">
										{#each all.slice(0, all.length - 1) as q}
											<Quantity quantity={q} /><span class="text-base-12">{', '}</span>
										{/each}
										<Quantity quantity={all[all.length - 1]} />
									</span>
								{/if}
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
					{#each recipe.cookware as item}
						<li>
							<span class="capitalize">{item.name}</span>{#if item.quantity}
								: <span class="text-base-11">{qValueFmt(item.quantity)}</span>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	<div class="flex flex-wrap justify-between">
		<h2 class="text-2xl my-2 font-heading">Method</h2>
		<div class="flex">
			<Listbox value={$stepIngredientsView} on:change={(e) => stepIngredientsView.set(e.detail)}>
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
	{/each}
</div>

<style>
	.content :global([data-ingredient-ref-group]) {
		--at-apply: bg-transparent outline-primary-6 outline-2 transition-colors px-1 -mx-1 py-1 -my-1
			rounded;
	}
	.content :global(.highlight) {
		--at-apply: bg-primary-4 outline relative z-1;
	}
</style>
