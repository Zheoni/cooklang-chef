export type Recipe = {
	name: string;
	metadata: Metadata;
	sections: Section[];
	ingredients: Ingredient[];
	cookware: Cookware[];
	timers: Timer[];
	inline_quantities: Quantity[];
	grouped_ingredients: IngredientListEntry[];
	timers_seconds: (Value | null)[];
	filtered_metadata: [string, string][];
	external_image: string | null;
	data: Scale;
};

export type Metadata = {
	description: string | null;
	tags: string[];
	emoji: string | null;
	author: NameAndUrl | null;
	source: NameAndUrl | null;
	time: RecipeTime | null;
	servings: number[] | null;
	map: Record<string, string>;
};
export type NameAndUrl = { name: string | null; url: string | null };
export type RecipeTime = number | { prep_time: number | null; cook_time: number | null };

export type Section = {
	name: string | null;
	steps: Step[];
};
export type Step = {
	items: Item[];
	number: number | null;
};
export type Item =
	| { type: 'text'; value: string }
	| { type: 'ingredient'; index: number }
	| { type: 'cookware'; index: number }
	| { type: 'timer'; index: number }
	| { type: 'inlineQuantity'; index: number };

export type Ingredient = {
	name: string;
	alias: string | null;
	quantity: Quantity | null;
	note: string | null;
	modifiers: string;
	relation: IngredientRelation;
};
export type Cookware = {
	name: string;
	alias: string | null;
	quantity: Value | null;
	note: string | null;
	modifiers: string;
	relation: ComponentRelation;
};
export type Timer = {
	name: string | null;
	quantity: Quantity;
};

export type Quantity = {
	value: Value;
	unit: string | null;
};

export type Value =
	| { type: 'number'; value: number }
	| { type: 'range'; value: { start: number; end: number } }
	| { type: 'text'; value: string };

export type IngredientListEntry = {
	index: number;
	quantity: Quantity[];
	outcome: ScaleOutcome | null;
};

export type ComponentRelation =
	| {
			type: 'definition';
			referenced_from: number[];
	  }
	| { type: 'reference'; references_to: number };

// I don't know enough typescript to do with without duplicating the code
export type IngredientRelation =
	| {
			type: 'definition';
			referenced_from: number[];
	  }
	| {
			type: 'reference';
			references_to: number;
			reference_target: 'ingredient' | 'step' | 'section';
	  };

export type PhysicalQuantity = 'volume' | 'mass' | 'length' | 'temperature' | 'time';

export type ScaleOutcome = 'Scaled' | 'Fixed' | 'NoQuantity' | 'Error';

export type Scale =
	| {
			type: 'DefaultScaling';
	  }
	| {
			type: 'Scaled';
			target: {
				base: number;
				index: number | null;
				target: number;
			};
			ingredients: ScaleOutcome[];
			cookware: ScaleOutcome[];
			timers: ScaleOutcome[];
	  };

/* Other */
export type Image = { path: string; indexes: { section: number; step: number } | null };

/* Reports */
export type Report<T> = {
	value: T | null;
	warnings: string[];
	errors: string[];
	fancy_report: string | null;
};
