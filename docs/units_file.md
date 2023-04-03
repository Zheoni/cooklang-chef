# Units file
Units files are a way to add new units or customise already defined units.

The main use for a custom units file is translation to other languages.

## Using them
Units files are layered. You can use as many as you want, but usage order
matters, as a file added later can override configuration set by an older one.

If the bundled units are enabled (which are by default), unit files are layered
on top of those.

In the CLI you can add as many as you want with the configuration file or
passing the `--units` flags as many times as you want.

## Format
The units file are stored in [TOML](https://toml.io) format. You can see an
example of how to structure this file in the
[units.toml](../cooklang/units.toml) file. These units are the bundled units,
they are defined in the same way as you can define your own units.

### `default_system`
This is the unit system used as a fallback in some parts of the program.
Possible values are:
- `metric` (default)
- `imperial`

### `si`
Here the International System of Units (SI) expansions are defined. To avoid
repetition, when defining a unit from the SI, you only have to define the base
unit and mark it with `expand_si = true`. Then the *kilo*, *centi*, etc. units
will be automatically defined.

Inside `si` there are 3 options:
- `prefixes`: define the name prefixes For example:
    ```toml
    [si.prefixes]
    kilo = ["kilo"]
    hecto = ["hecto"]
    deca = ["deca"]
    deci = ["deci"]
    centi = ["centi"]
    milli = ["milli"]
    ```
    Notice that these are arrays, so you can have more than one prefix per base.

- `symbol_prefixes`: define the symbol prefixes For example:
    ```toml
    [si.symbol_prefixes]
    kilo = ["k"]
    hecto = ["h"]
    deca = ["da"]
    deci = ["d"]
    centi = ["c"]
    milli = ["m"]
    ```
- `precedence`: When layering unit files, how the prefixes should be combined.
    - `before` (default): add prefixes before the older layers.
    - `after`: add prefixes after the older layers.
    - `override`: replace older layer prefixes.

    Order matters because when formatting units, the first name/symbol will be
    used.

### `extend`
Extend units from other layers. Options inside:
- `precedence`. Same as `si.precence` but for the names/symbols/aliases defined
here.
- `names`. Map from any name, symbol or alias of an already defined unit
  (probably in other unit file) to more names.
- `symbols`. Same as `names` but for symbols.
- `aliases`. Same as `names` but for aliases. Aliases are used to parse a unit,
  but is the last option when printing them.

### `quantity`
With [toml array of tables](https://toml.io/en/v1.0.0#array-of-tables) this can
be defined more than once. Each appearance defines units for a different
physical quantity.

Options inside:
- `quantity`. Physical quantity of the following units. Possible values:
    - `volume`
    - `mass`
    - `length`
    - `temperature`
    - `time`
- `best`. Define the units elegible to be the first conversion for a system.
    This can either be a list or a table with 2 lists, one for metric and one
    for imperial.
- `units`. List of unit definitions or map with 2 lists, one for metric units
    and another for imperial units. If just a list is used, the units will not
    have a system assigned to them.[^1]

    See in [units](####`units`)

[^1]: Except it they appear in a specific unit sytem in the `best` field, then
    it will inherit the system.

#### Units
Each unit entry is a map with:
- `names`: list of names
- `symbols`: list of symbols
- `aliases`: list of aliases
- `ratio`: conversion ratio, it is best practice to have the *base* unit of the
quantity (for example, *gram* for mass) to have ratio of `1.0`.
- `difference`: offset a conversion after applying the ratio, useful for not
direct conversions like temperature.
- `expand_si`: flag that if enable, generates units with the prefixes defined in
  `si`.


This document may be really confusing, I recommend reading the
[default units file](../cooklang/units.toml) or any of the extra files
[here](../cooklang/units/).