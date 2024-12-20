# `chef` Change Log

## Unreleased - ReleaseDate

## 0.9.2 - 2024/12/20

This release is the last one before updating cooklang with spec changes. It
includes a variety of small fixes and improvements.

- Improve number formatting in the web ( @StarDylan #30 )
- Expand search bar capabilities ( @kaylee-kiako #32 )
- Fixes related to real time file system updates ( @kaylee-kiako #39 )

## 0.9.1 - 2024/04/18

- Fix `VISUAL` and/or `EDITOR` env vars that were ignored. ( #26 )

## 0.9.0 - 2024/04/11

- Add more markdown customization to change all hard coded words.
- Add `--config` global arg to override the `config.toml` file used.

## 0.8.5 - 2024/02/27

- Don't allow `serve` or `list` to run outside a collection to avoid unwanted
  recipe indexing.
- Add `--force` arg to `list` to force it to run outside a collection.
