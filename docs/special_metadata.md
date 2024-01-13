# Special metadata keys

If `chef` some metadata keys can have special meaning. Using these will allow
`chef` to extract more information from the recipe and/or customize the
behaviour.

- `name` will override the recipe name. Instead of the file name, the value of
  the key will be used.

- `tags` comma separated list of tags.

- `emoji` adds an emoji that matches the recipe. It has to be an emoji or a
  shortcode like `:taco:`.

- `description` recipe description.

- `author` stores *who* wrote the recipe. It stores a name and an URL. It can have
  one of these formats:

  - `name`, like `Rachel`
  - `URL`
  - `name <URL>`, like `Rachel <herwebsite.whatever>`

- `source` stores *where* the recipe was obtained from. Same format as `author`.

- `time` total recipe time. Overrides `prep_time` and `cook_time` if after them.

- `prep_time` preparation time. Overrides `time` if after it.

- `cook_time` cooking time. Overrides `time` if after it.
