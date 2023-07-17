<script lang="ts">
	import { fly } from 'svelte/transition';
	import { timer, remainingSeconds, removeTimer } from './timer';

	import Play from '~icons/lucide/play';
	import Pause from '~icons/lucide/pause';
	import IconX from '~icons/lucide/x';

	let audio: HTMLAudioElement;

	function formatSeconds(secs: number | null) {
		if (secs === null) return '';

		let minutes = Math.trunc(secs / 60);
		secs %= 60;
		let hours = Math.trunc(minutes / 60);
		minutes %= 60;

		const parts = [];
		if (hours > 0) {
			parts.push(hours.toString());
		}
		parts.push(minutes.toString().padStart(2, '0'));
		parts.push(secs.toString().padStart(2, '0'));
		return parts.join(':');
	}

	$: if ($timer?.status === 'finished') {
		audio.play();
	}

	$: timeText = formatSeconds($remainingSeconds);
</script>

{#if $timer}
	<div
		class="sticky bottom-4 md:bottom-8 my-4 h-22 md:w-64 md:mx-auto min-w-64 rounded-xl p-2 shadow shadow-indigo-1 z-100 bg-indigo-3 overflow-hidden"
		class:fancy-border={$timer.status === 'running'}
		class:regular-border={$timer.status !== 'running'}
		class:finished={$timer.status === 'finished'}
		transition:fly={{ y: '50%' }}
	>
		{#if $timer.name}
			<div
				class="text-indigo-11 capitalize font-semibold text-center absolute top-1 left-0 right-0"
			>
				{$timer.name}
			</div>
		{/if}

		<div class="timer-grid h-full items-center justify-center space-x text-2xl">
			<div>
				{#if $timer.status === 'paused'}
					<button
						class="text-primary-9 flex items-center justify-end"
						on:click={() => {
							if ($timer) {
								$timer.start();
							}
						}}
					>
						<Play />
					</button>
				{:else if $timer.status === 'running'}
					<button
						class="text-primary-9 flex items-center justify-end"
						on:click={() => {
							if ($timer) {
								$timer.pause();
							}
						}}
					>
						<Pause />
					</button>
				{/if}
			</div>

			<div class="text-center">
				<span class="text-base-12 text-4xl select-none tabular-nums">{timeText}</span>
			</div>

			<button
				class="text-red-9 flex items-center"
				on:click={() => {
					if ($timer) {
						removeTimer();
					}
				}}
			>
				<IconX />
			</button>
		</div>
	</div>
{/if}
<audio src="/mixkit-alarm-tone-996.wav" bind:this={audio} preload="none" />

<style>
	.timer-grid {
		display: grid;
		grid-template-columns: 1fr 5fr 1fr;
	}

	.fancy-border::before {
		content: '';
		position: absolute;
		z-index: -2;
		min-height: 150%;
		min-width: 150%;
		aspect-ratio: 1;
		top: -150px;
		left: -65px;
		background-repeat: no-repeat;
		background-position: 0 0;
		background-image: conic-gradient(transparent, hsla(226, 99.9%, 63.6%, 0.848), transparent 30%);
		animation: rotate 4s linear infinite;
		--at-apply: 'bg-indigo-6';
	}

	.fancy-border::after {
		content: '';
		position: absolute;
		z-index: -1;
		inset: 4px;
		border-radius: 10px;
		--at-apply: 'bg-indigo-3';
	}

	@keyframes rotate {
		100% {
			transform: rotate(1turn);
		}
	}

	.regular-border::before {
		content: '';
		position: absolute;
		z-index: -2;
		inset: 0;
		--at-apply: 'bg-indigo-6';
	}

	.regular-border::after {
		content: '';
		position: absolute;
		z-index: -1;
		inset: 4px;
		border-radius: 10px;
		--at-apply: 'bg-indigo-3';
	}

	.finished {
		animation: shake 0.5s ease-in-out;
	}
	.finished::before {
		--at-apply: 'bg-red-6';
	}
	.finished::after {
		--at-apply: 'bg-red-3';
	}

	@keyframes shake {
		0% {
			rotate: 0;
		}
		20% {
			rotate: 10deg;
		}
		40% {
			rotate: -10deg;
		}
		60% {
			rotate: 10deg;
		}
		80% {
			rotate: -10deg;
		}
		100% {
			rotate: 0;
		}
	}
</style>
