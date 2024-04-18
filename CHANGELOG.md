# `chef` Change Log

## Unreleased - ReleaseDate

## 0.9.1 - 2024/04/18

- Fix `VISUAL` and/or `EDITOR` env vars that were ignored. ( #26 )

## 0.9.0 - 2024/04/11

- Add more markdown customization to change all hard coded words.
- Add `--config` global arg to override the `config.toml` file used.

## 0.8.5 - 2024/02/27

- Don't allow `serve` or `list` to run outside a collection to avoid unwanted
  recipe indexing.
- Add `--force` arg to `list` to force it to run outside a collection.
