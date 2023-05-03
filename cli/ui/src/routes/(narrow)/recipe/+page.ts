import { error, redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { API } from '$lib/constants';
import type { Recipe, Report, Image } from '$lib/types';
import { is_valid } from '$lib/util';

type Resp = {
	recipe: Report<Recipe>;
	src_path: string;
	images: Image[];
	created: number | null;
	modified: number | null;
};

export const load = (async ({ fetch, url }) => {
	const path = url.searchParams.get('r');
	if (path === null) {
		throw redirect(301, '/');
	}
	const apiParams = new URLSearchParams();
	const scale = url.searchParams.get('scale');
	if (scale) apiParams.set('scale', scale);
	const units = url.searchParams.get('units');
	if (units) apiParams.set('units', units);
	const apiUrl = `${API}/recipe/${path}?${apiParams}`;
	const resp = await fetch(apiUrl);
	return await fetchRecipe(resp);
}) satisfies PageLoad;

async function fetchRecipe(resp: Response) {
	if (resp.status === 404) {
		throw error(404, 'Recipe not found');
	}
	if (!resp.ok) {
		throw error(400);
	}
	const data = (await resp.json()) as Resp;
	if (!is_valid(data.recipe)) {
		throw error(500, {
			message: 'Error parsing recipe',
			code: 'PARSE',
			srcPath: data.src_path,
			images: data.images,
			parse_errors: data.recipe.errors,
			parse_warnings: data.recipe.warnings,
			fancy_report: data.recipe.fancy_report
		});
	}
	return {
		recipe: data.recipe.value!,
		warnings: data.recipe.warnings,
		srcPath: data.src_path,
		images: data.images,
		created: data.created,
		modified: data.modified,
		fancy_report: data.recipe.fancy_report
	};
}
