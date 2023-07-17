import { toast } from '@zerodevx/svelte-toast';

export function success(message: string) {
	toast.push(message, {
		theme: {
			'--toastBackground': 'var(--un-preset-radix-grass9, green)',
			'--toastColor': 'white',
			'--toastBarBackground': 'hsl(120, 53.6%, 94.8%)'
		}
	});
}

export function error(message: string) {
	toast.push(message, {
		theme: {
			'--toastBackground': 'var(--un-preset-radix-tomato9, salmon)',
			'--toastColor': 'white',
			'--toastBarBackground': 'hsl(8, 100%, 96.6%)'
		}
	});
}

export function timer(message: string) {
	toast.push(message, {
		theme: {
			'--toastBackground': 'var(--un-preset-radix-indigo9, salmon)',
			'--toastColor': 'white',
			'--toastBarBackground': 'hsl(223, 98.4%, 97.1%)'
		}
	});
}

export default { success, error, timer };
