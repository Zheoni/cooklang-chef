# cooklang

[![Crates.io](https://img.shields.io/crates/v/cooklang)](https://crates.io/crates/cooklang)
[![docs.rs](https://img.shields.io/docsrs/cooklang)](https://docs.rs/cooklang/)
![Crates.io](https://img.shields.io/crates/l/cooklang)

Cooklang parser in rust with opt-in extensions.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like the extensions.

You can see a detailed list of all extensions explained [here](../docs/extensions.md).

Entire `cooklang-rs` project: https://github.com/Zheoni/cooklang-rs