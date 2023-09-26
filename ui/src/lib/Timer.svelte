<script lang="ts">
	import { tooltip } from 'svooltip';
	import Quantity from './Quantity.svelte';
	import type { Timer, Value } from './types';
	import { timer as recipeTimer, setTimer } from './timer';
	import { t } from './i18n';

	export let timer: Timer;
	export let seconds: Value | null;

	$: isTimerSet = $recipeTimer !== null && $recipeTimer.status !== 'finished';
	$: setTimerAvailable = !isTimerSet && seconds !== null;

	function handleClick() {
		if (!setTimerAvailable || seconds === null) return;
		let secs;
		if (seconds.type === 'number') {
			secs = seconds.value;
		} else if (seconds.type === 'range') {
			// TODO let user choose maybe
			secs = seconds.value.start;
		} else {
			console.error('Text value in timer');
			return;
		}

		const t = setTimer(secs, timer.name ?? undefined);
		t.start();
	}
</script>

<button
	class="inline text-indigo-11 font-semibold"
	class:hover:underline={setTimerAvailable}
	use:tooltip={{ content: $t('timer.start'), visibility: setTimerAvailable }}
	on:click={handleClick}
	disabled={!setTimerAvailable}
>
	{timer.name ?? ''}
	<Quantity quantity={timer.quantity} />
</button>
