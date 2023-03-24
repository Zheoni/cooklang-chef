# cooklang-rs

> ⚠️ Every part of this repo is still work in progress and may change at any time

A superset of [cooklang](https://cooklang.org/) and related tools.

## What is this
I wanted a couple more feature that cooklang did not have, so I extended the
cooklang syntax and semantics a bit.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like the extensions.

You can see a detailed list of all extensions explained [here](./docs/extensions.md).

Some key features:
- **Ingredient references**. You can now refer to ingredients you already used
  before. I think the most important extension to the original cooklang. You can
  read about using references in [this document](./docs/using_references.md).
- **Good error reporting**. Error reports are a top priority. Specially those 
  related to parsing cooklang files.

  This little recipe contain errors:
  ```cooklang
  >> servings: 3|6|8

  Add @water{1%kg}, mix, and ~{5 min} later add more @&water{1|2%L}.
  ```
  ![](./images/error_report.png)

- **Units**. An ingredient quantity means nothing without a units. This is why
  the units are parsed a and checked. With units come:
  - Unit conversion. You can read your recipe in your prefered unit system.
  - Configurable units. You can add, remove and rename units.

## Crates

- [Cooklang parser](./cooklang/)
- [The CLI](./cli/). The CLI readme details it features and explains how to
  compile it.
- [cooklang-fs](./cooklang-fs). Utilities to deal with referencing recipe, 
  images and data related to recipes that are in other files.
- [cooklang-to-cooklang](./cooklang-to-cooklang). Recipe back to Cooklang.
- [cooklang-to-human](./cooklang-to-human). Write a recipe in a human friendly way.
- [cooklang-to-md](./cooklang-to-md). Recipe into Markdown.
