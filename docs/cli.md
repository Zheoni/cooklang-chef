# The `cooklang-rs` CLI

## Key features
- Read a recipe:
    ```sh
    chef recipe Bread.cook
    ```
    ![](../images/bread3.png) You can also specify a `markdown`, `json` or back
    to `cooklang` output.

- List all recipes, even check if they contain errors.
    ```sh
    chef list -l
    ```
    ![](../images/list.png)

- Collections. You don't have to be in any specific directory to access the
  recipes. A default collection can be set and use anywhere in the system.

- Quick conversions
    ```sh
    chef convert 3 cups metric
    ```

- Web UI
    ```sh
    chef serve --open
    ```
    This starts a web server and opens it in the default web browser. You can
    edit the recipe files and it will automatically update the web on save.

    ![](../images/webui.png)
    
    This is intended for personal or home use for a because:
    - No strict protection is used.
    - There is no caching, so every request the recipe file is read from the
    disk and parsed.

## Installing
### Install with cargo
```sh
cargo install --git https://github.com/Zheoni/cooklang-chef/ --tag "v0.10.0" --locked
```
This will automatically download and compile the CLI.

### Prebuilt binaries
Binaries are provided with the [Github
releases](https://github.com/Zheoni/cooklang-chef/releases).

### Manually compiling the CLI
1. Install the rust compiler and `cargo`, the best way is with
   [rustup](https://rustup.rs/).
2. Clone [this repo](https://github.com/Zheoni/cooklang-rs).
    ```sh
    git clone https://github.com/Zheoni/cooklang-rs
    cd cooklang-rs
    ```
3. To install it run **ONE** of the following:
    ```sh
    # enable everything
    cargo install --path .

    # no `serve` cmd
    cargo install --path . --no-default-features
    ```

    This will install the cli in the `cargo` install dir, in your home dir. If
    you followed the instructions when using `rustup`, this dir should be in
    your `PATH` and the binary accesible.

4. Test it:
    ```sh
    chef help
    ```
    This prints a usage guide.

    It is also recomended to run the interactive setup if it's the first time
    using `chef`.
    ```sh
    chef config --setup
    ```

## Configuration
A configuration [TOML](https://toml.io) file will be loaded by the CLI. First,
it will try to load it from `.cooklang/config.toml`, if that cannot be found, a
global default configuration file will be loaded. If it does not exist either,
it will use default values.

The configuration file can be overriden with the CLI args.

You can see the loaded configuration with:
```sh
# collection config
chef config

# global config
chef config --chef
```

The global configuration that stores configuration of `chef` itself and not
specific to a collection.

### The configuration file
This is the default configuration. You only need to set the fields that you want
to change.

```toml
default_units = true             # use bundled units
warnings_as_errors = false       # treat any warning as an error
recipe_ref_check = true          # check recipe references
max_depth = 10                   # max depth to search for recipe references

# enabled extensions
# this can also be `extensions = "all"` or `extensions = "none"`
[extensions]
COMPONENT_MODIFIERS = true
COMPONENT_ALIAS = true
ADVANCED_UNITS = true
MODES = true
INLINE_QUANTITIES = true
RANGE_VALUES = true
TIMER_REQUIRES_TIME = true
INTERMEDIATE_PREPARATIONS = true

# load is used to tell chef to load extra configuration files
# * the default is empty, but see below
[load] 
units = ["path/to/a/units.toml"] # load extra units files
aisle = "path/to/aisle.conf"     # load aisle.conf

# configuration of the web ui (currently only tags emojis)
[ui.tags]
mexican = { emoji = ":taco:" }   # * the default is emtpy

# export format configuration (currently only markdown)
[export.markdown]
tags = true                      # show tags
description = "blockquote"       # or "heading" or "hidden"
escape_step_numbers = false      # everything is a paragraph
italic_amounts = true            # put amounts in italics
front_matter_name = "name"       # key "name" in the frontmatter with the recipe name
heading.section = "Section %n"   # used in sections without name. `%n` is the section number
heading.ingredients = "Ingredients"
heading.cookware = "Cookware"
heading.steps = "Steps"
heading.description = "Description" # used when `description = "heading"
optional_marker = "(optional)"
```

The paths in `load`, if relative, they are relative from the `.cooklang` dir.

If no units `load.units` is given, `chef` will try to load
`.cooklang/units.toml`. If that fails, it will try to load a global `units.toml`
file stored alongside the global config, run `chef config --chef` to see where
is that.

Same thing happens with `load.aisle`, it will try to load an `aisle.conf` file
automatically.
