# cooklang-rs

A superset of [cooklang](https://cooklang.org/) and related tools.

## What is this
I wanted a couple more feature that cooklang did not have, so I extended the
cooklang syntax and semantics a bit.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like the extensions.

You can see a detailed list of all extensions explained [here](./docs/extensions.md).

## Crates

- [Cooklang parser](./cooklang/)
- [The CLI](./cli/)
- [cooklang-fs](./cooklang-fs)
- [cooklang-to-cooklang](./cooklang-to-cooklang)
- [cooklang-to-human](./cooklang-to-human)
- [cooklang-to-md](./cooklang-to-md)