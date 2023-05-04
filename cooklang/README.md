# cooklang

Cooklang parser in rust with opt-in extensions.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like the extensions.

You can see a detailed list of all extensions explained [here](../docs/extensions.md).

Entire `cooklang-rs` project: https://github.com/Zheoni/cooklang-rs