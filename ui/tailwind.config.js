const radixColors = require("@radix-ui/colors");
const { createPlugin: colorsPlugin } = require("windy-radix-palette");
const {
  iconsPlugin,
  getIconCollections,
} = require("@egoist/tailwindcss-icons");
const defaultTheme = require("tailwindcss/defaultTheme");

const colors = colorsPlugin({
  colors: {
    olive: radixColors.olive,
    oliveDark: radixColors.oliveDark,
    grass: radixColors.grass,
    grassDark: radixColors.grassDark,
    blue: radixColors.blue,
    blueDark: radixColors.blueDark,
    indigo: radixColors.indigo,
    indigoDark: radixColors.indigoDark,
    yellow: radixColors.yellow,
    yellowDark: radixColors.yellowDark,
    tomato: radixColors.tomato,
    tomatoDark: radixColors.tomatoDark,
    orange: radixColors.orange,
    orangeDark: radixColors.orangeDark,
    sage: radixColors.sage,
    sageDark: radixColors.sageDark,
    green: radixColors.green,
    greenDark: radixColors.greenDark,
    jade: radixColors.jade,
    jadeDark: radixColors.jadeDark,
    sand: radixColors.sand,
    sandDark: radixColors.sandDark,
  },
});

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["templates/**/*.html", "assets/js/**/*.js"],
  theme: {
    container: {
      center: true,
    },
    extend: {
      fontFamily: {
        sans: ['"Noto Sans"', ...defaultTheme.fontFamily.sans],
        serif: ['"Noto Serif"', ...defaultTheme.fontFamily.serif],
        mono: ['"JetBrains Mono"', ...defaultTheme.fontFamily.mono],
        heading: ["Typey", "serif"],
      },
      colors: {
        primary: colors.alias("grass"),
        base: colors.alias("olive"),
        green: colors.alias("grass"),
        red: colors.alias("tomato"),
      },
    },
  },
  plugins: [
    colors.plugin,
    iconsPlugin({
      collections: getIconCollections(["lucide"]),
      scale: 1.2,
      extraProperties: {
        "vertical-align": "-0.1em",
      },
    }),
  ],
  darkMode: "class",
  safelist: [
    "bg-red-3",
    "bg-yellow-3",
    "border-yellow-6",
    "border-red-6",
    "border-red-7",
    "border-green-7",
    "bg-green-5",
    "bg-red-5",
  ],
};
