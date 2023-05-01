import type { Report } from './types';

export function is_valid<T>(report: Report<T>) {
	return report.value !== null && report.errors.length === 0;
}
