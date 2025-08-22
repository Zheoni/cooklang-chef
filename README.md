# cooklang-chef

A CLI to manage [cooklang](https://cooklang.org/) recipes with extensions.

> The parser has been moved to [cooklang/cooklang-rs](https://github.com/cooklang/cooklang-rs)

> `0.10.x` will be the last version of `chef` published on `crates.io`.

> `0.15.x` will be the last versions of `cooklang-fs`, `cooklang-to-cooklang`
> `cooklang-to-human` and `cooklang-to-md` published on `crates.io`.

## What is cooklang
Cooklang is a markup language for cooking recipes. An in depth explanation can
be found in [cooklang.org](https://cooklang.org/).

An example cooklang recipe:
```cooklang
In a large #bowl mix @flour{450%g}, @yeast{2%tsp}, @salt{2%tsp} and
@warm water{375%ml}.

Cover the dough and leave on counter for ~{2-3%hour}.

Sprinkle work surface with @&flour{10%g} and shape the dough. Sprinkle the top
with some more @&flour{5%g}.

Bake with a preheated #oven at 230ºC for ~{30%min}.
```
![](./images/bread3.png)

## What is cooklang-chef
`chef` is a CLI to manage, read and convert cooklang recipes.

I wanted a couple more features that cooklang did not have, so I extended the
cooklang syntax and semantics a bit.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like the extensions. All extensions except the multiline steps
are enabled by default[^1].

[^1]: This is done to maximize compatibility with other cooklang parsers.

You can see a detailed list of all extensions explained in [the parser repo](https://github.com/cooklang/cooklang-rs/blob/main/extensions.md).

Full user documentation [here](./docs/README.md).

You can install `chef` with:
```sh
cargo install --git https://github.com/Zheoni/cooklang-chef/ --tag "v0.10.1" --locked
```

You can also get a prebuilt binary from the github releases.

After installing it, run:
```sh
chef config --setup
```

Key features:

- **Web UI**. The [CLI](./docs/cli.md) comes with an embedded web UI.
  - Scale and convert the quantities.
  - Hot reload of recipes. Just edit the `.cook` file and save.
  - Open the `.cook` file in a code editor.
  
  ![](./images/webui.png)

- **Ingredient references**. You can now refer to ingredients you already used
  before. I think the most important extension to the original cooklang. You can
  read about using references in [this document](./docs/using_references.md).

- **Good error reporting**. Error reports are a top priority.

  This little recipe contain errors:
  ```cooklang
  >> servings: 3|6|8

  Add @water{1%kg}, mix, and ~{5 min} later add more @&water{1|2%L}.
  ```
  ![](./images/error_report.png)

- **Units**. An ingredient quantity means nothing without a unit. This is why
  the units are parsed and checked. With units come:
  - Unit conversion. You can read your recipe in your prefered unit system.
  - Configurable units. You can add, remove and rename units.

## Crates

- [Cooklang parser](https://github.com/cooklang/cooklang-rs) [![Crates.io](https://img.shields.io/crates/v/cooklang)](https://crates.io/crates/cooklang) [![docs.rs](https://img.shields.io/docsrs/cooklang)](https://docs.rs/cooklang/)
- The CLI [![Crates.io](https://img.shields.io/crates/v/cooklang-chef)](https://crates.io/crates/cooklang-chef)
- [cooklang-fs](./cooklang-fs). [![Crates.io](https://img.shields.io/crates/v/cooklang-fs)](https://crates.io/crates/cooklang-fs)
  Utilities to deal with referencing recipe, images and data related to recipes that are in other files.
- [cooklang-to-cooklang](./cooklang-to-cooklang). [![Crates.io](https://img.shields.io/crates/v/cooklang-to-cooklang)](https://crates.io/crates/cooklang-to-cooklang) Recipe back to Cooklang.
- [cooklang-to-human](./cooklang-to-human). [![Crates.io](https://img.shields.io/crates/v/cooklang-to-human)](https://crates.io/crates/cooklang-to-human) Write a recipe in a human friendly way.
- [cooklang-to-md](./cooklang-to-md). [![Crates.io](https://img.shields.io/crates/v/cooklang-to-md)](https://crates.io/crates/cooklang-to-md) Recipe into Markdown.
