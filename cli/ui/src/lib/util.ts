import type { Report } from './types';

export function is_valid<T>(report: Report<T>) {
	return report.value !== null && report.errors.length === 0;
}

export function displayName<T extends { name: string; alias: string | null }>(component: T) {
	return component.alias ?? component.name;
}
