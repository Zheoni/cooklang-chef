# `chef` Change Log

## Unreleased - ReleaseDate

- Follow symbolic links when scanning recipes directory ( #59 )
- Added `Dockerfile` and `docker-compose.yml` to the repository ( #58 #62 )

## 0.10.2 - 2025/10/14

- Fix HTML render fail when only prep time was given without cook time or vice
  versa. ( #55 )

## 0.10.1 - 2025/04/21

- Add french translation ( @ornicar #50 )

## 0.10.0 - 2025/01/14

Updates to cooklang parser `0.15.0`, this includes many small improvements and
some changes to the language. See the [parser changelog] from `0.13` to `0.15`
to see the changes. Major things you neeed to worry about are:

- If you used a custom extensions config in your recipe collection, you may need
  to update it because some extensions have been removed (always enabled) and
  some renamed.
- The metadata now uses a YAML frontmatter, you will get a warning if you use
  the old style. You can't mix both styles of metadata, when you use a
  frontmatter the old style syntax will be disabled. Config keys like `>> [mode]: ...`
  will still use the old style and are not metadata anymore.

New features this enables:

- The YAML frontmatter for metadata.
- Now all inline quantities are detected and can be converted at your will, not
  only temperature.

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
