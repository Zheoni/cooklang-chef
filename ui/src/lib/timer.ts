import { get, readable, writable, type Readable, derived } from 'svelte/store';

type TimerData = {
	name?: string;
	end: Date;
};

export const timer = writable<null | RecipeTimer>(null);

class RecipeTimer {
	name?: string;
	seconds: number;
	end: Date | null;
	timeoutId: number | null;
	status: TimerStatus;
	notification: Notification | null;

	constructor(seconds: number, name?: string) {
		this.seconds = seconds;
		this.name = name;
		this.end = null;
		this.timeoutId = null;
		this.status = 'paused';
		this.notification = null;
	}

	start() {
		if (this.status === 'finished') return;
		const end = new Date();
		end.setSeconds(end.getSeconds() + this.seconds);
		this.end = end;

		const secs = Math.round(dateDiffSecs(new Date(), end));
		this.timeoutId = setTimeout(() => {
			this.status = 'finished';
			this.seconds = 0;
			this.timeoutId = null;

			timer.update((v) => v);
		}, secs * 1000);
		this.status = 'running';
		timer.update((v) => v);
	}

	pause() {
		if (this.status === 'finished') return;
		if (this.timeoutId) clearTimeout(this.timeoutId);
		this.timeoutId = null;

		if (this.end) {
			const secs = Math.round(dateDiffSecs(new Date(), this.end));
			this.seconds = secs;
			this.end = null;
		}
		this.status = 'paused';
		timer.update((v) => v);
	}

	destroy() {
		this.status = 'finished';
		if (this.timeoutId) clearTimeout(this.timeoutId);
		this.timeoutId = null;
		this.end = null;
		if (this.notification) this.notification.close();
		this.notification = null;
		this.seconds = 0;
	}

	static remainingSeconds() {
		return remainingSeconds;
	}
}

type TimerStatus = 'paused' | 'running' | 'finished';

export function setTimer(seconds: number, name?: string) {
	const rt = new RecipeTimer(seconds, name);

	timer.update((t) => {
		t?.destroy();
		return rt;
	});

	return rt;
}

export function removeTimer() {
	timer.update((t) => {
		t?.destroy();
		return null;
	});
}

export const remainingSeconds: Readable<number | null> = derived(timer, ($timer, set) => {
	if ($timer === null) {
		set(null);
		return;
	}

	if ($timer.status === 'finished') {
		set(0);
		return;
	}

	if ($timer.status === 'paused') {
		set($timer.seconds);
	}

	const interval = setInterval(() => {
		if ($timer.end) {
			const secs = dateDiffSecs(new Date(), $timer.end);
			set(Math.max(0, Math.round(secs)));
		} else {
			set($timer.seconds);
		}
	}, 100);

	return () => clearInterval(interval);
});

// let autoGenCounter = 0;
// export function createTimer(seconds: number, name: string | null): TimerData {
// 	if (name === null) {
// 		const t = get(_timers);
// 		name = 'Timer';
// 		while (Object.hasOwn(t, name)) {
// 			autoGenCounter += 1;
// 			name = `Timer ${autoGenCounter}`;
// 		}
// 	}

// 	const start = new Date();
// 	const end = new Date(start);
// 	end.setSeconds(end.getSeconds() + seconds);

// 	return {
// 		name,
// 		start,
// 		end,
// 		remainingSeconds: seconds,
// 		startSeconds: seconds
// 	};
// }

// export function addTimer(timer: TimerData) {
// 	const timerStore = readable(timer, (_set, update) => {
// 		const updateRemainigSeconds = () => {
// 			update((t) => {
// 				t.remainingSeconds = dateDiffSecs(new Date(), t.end);
// 				return t;
// 			});
// 		};

// 		updateRemainigSeconds();

// 		const interval = setInterval(updateRemainigSeconds, 500);

// 		return () => clearInterval(interval);
// 	});

// 	_timers.update((t) => {
// 		t[timer.name] = timerStore;
// 		return t;
// 	});

// 	return timerStore;
// }

function dateDiffSecs(start: Date, end: Date) {
	const diffMillis = end.getTime() - start.getTime();
	return diffMillis / 1000;
}
