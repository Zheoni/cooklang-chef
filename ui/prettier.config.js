module.exports = {
  plugins: ["prettier-plugin-tailwindcss", "prettier-plugin-jinja-template"],
  overrides: [
    {
      files: ["*.html"],
      options: {
        parser: "jinja-template",
      },
    },
  ],
};
