import { defineConfig } from 'unocss/vite';
import { presetWebFonts, presetIcons, presetUno, transformerDirectives } from 'unocss';
import { presetRadix } from 'unocss-preset-radix';
import { presetForms } from '@julr/unocss-preset-forms';

export default defineConfig({
	shortcuts: [
		{
			btn: 'font-semibold rounded active:translate-y-1px inline-block'
		},
		{
			'btn-square': 'font-semibold rounded active:translate-y-1px inline-grid place-items-center'
		},
		[/^btn-square-(\d+)$/, ([, s]) => `btn-square h-${s} w-${s}`],
		[/^radix-solid-(.*)$/, ([, c]) => `bg-${c}-9 hover:bg-${c}-10 text-white`],
		{
			link: 'text-primary-11 hover:underline'
		},
		{
			code: 'font-mono px-2 py-1 bg-base-3 rounded border border-base-6'
		}
	],
	transformers: [transformerDirectives()],
	presets: [
		presetUno(),
		presetIcons({
			prefix: 'i-',
			extraProperties: {
				display: 'inline-block',
				'vertical-align': '-0.1em'
			},
			scale: 1.2
		}),
		presetWebFonts({
			provider: 'google',
			fonts: {
				serif: {
					name: 'Source Serif Pro',
					italic: true,
					weights: [400, 600, 700]
				},
				mono: 'JetBrains Mono',
				heading: [
					{
						name: 'Erode',
						weights: [600],
						provider: 'fontshare'
					},
					{ provider: 'none', name: 'serif' }
				]
			}
		}),
		presetRadix({
			palette: ['olive', 'grass', 'blue', 'indigo', 'yellow', 'tomato', 'orange'],
			aliases: {
				primary: 'grass',
				base: 'olive',
				green: 'grass',
				red: 'tomato'
			},
			darkSelector: '.dark',
			lightSelector: '.light'
		})
	],
	preflights: [
		{
			getCSS: () => `
				a, button, [role=button], input, label, select, summary, text-area, area {
					touch-action: manipulation;
				}
			`
		}
	],
	safelist: []
});
