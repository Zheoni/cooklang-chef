# Cooklang syntax extensions

## Modifiers
With the ingredient modifiers you can alter the behaviour of ingredients. There
are 5 modifiers:
- `@` **Recipe**. References another recipe by it's name.
  ```cooklang
  Add @@tomato sauce{200%ml}.
  ``` 
- `&` **Reference**. References another ingredient with the same name. If a
  quantity is given, the amount can be added. The ingredient must be defined
  before. If there are multiple definitions, use the last one.
  ```cooklang
  Add @flour{200%g} [...], then add more @&flour{300%g}.
  ```
- `-` **Hidden**. Hidden in the list, only appears inline.
  ```cooklang
  Add some @-salt.
  ```
- `?` **Optional**. Mark the ingredient as optional.
  ```cooklang
  Now you can add @?thyme.
  ```
- `+` **New**. Forces to create a new ingredient. This works with the 
  [modes](#modes) extension.

This also works (except recipe) for cookware.

## Component note
Simple, add small notes to ingredients. The notes in between parenthesis.

```coklang
@flour{}(all purpose)
@flour(all purpose)    -- can omit the close brackets
@flour{} (all purpose) -- ❌ no space between the ingredient and the note
```

This also works for cookware.

## Component alias
Add an alias to an ingredient to display a different name.

```cooklang
@white wine|wine{}
@@tomato sauce|sauce{}     -- works with modifiers too
```

This can be useful with references. Here, the references will be displayed as
`flour` even though the ingredient it's refering is `tipo zero flour`.

```cooklang
Add the @tipo zero flour{}
Add more @&tipo zero flour|flour{}
```

This also works for cookware.

## Sections
Divide the steps. Sections can have a name or not.

```cooklang
= Cooking         -- this
== Cooking ==     -- many before and after is also supported
====              -- without name
```

To add images to steps inside a section, add another index to the image name:
```txt
Recipe.0.jpeg   -- First section, first step
Recipe.0.0.jpeg -- The same
Recipe.1.0.jpeg -- Second section, first, step
```

## Text steps
All components are ignored and the steps don't increment the step counter. Some
people like to write a couple of paragraphs in the recipe that don't are steps.

It can also be used as notes that are not instructions.

```cooklang
Regular step.
> Text step.
```

## Multiline steps
In regular cooklang each line is a step. With this extension the division is by
a blank line in between, so:
```cooklang
A step,
the same step.

A different step.
```

## Advanced units
Maybe confusing name. Tweaks a little bit the parsing and behaviour of units
inside quantities.

- When the value is a number, the unit separator (`%`) can be replaced with a
a space.
  ```cooklang
  @water{1 L} is the same as @water{1%L}
  ```

  If disabeld, `@water{1 L}` would parse as `1 L` being a text value.
- Enables extra checks:
  - Checks that units between references are compatible, so they can be added.
  - Checks that timers have a time unit.

## Modes
Add new special metadata keys that control some of the other extensions. The
special keys are between square brackets.

```cooklang
>> [special key]: value
```

- `[mode]` | `[define]`
  - `all` | `default`. This is the default mode, same as the original cooklang.
  - `ingredients` | `components`. In this mode only components can be defined,
  all regular text is omitted. Useful for writing an ingredient list manually
  at the beginning of the recipe if you want to do so.
  - `steps`. All the ingredients are references. To force a new ingredient, use
  the new (`+`) modifier.
  - `text`. All steps are [text steps](#text-steps)

- `duplicate`
  - `new` | `default`. When a ingredient with the same name is found, create a
  new one. This is the original cooklang behaviour.
  - `reference` | `ref`. Ingredients have implicit references when needed. So
  ingredients with the same name will be references. To force a new ingredient,
  use the new (`+`) modifier.
    ```cooklang
    >> [duplicate]: ref
    @water{1} @water{2}
    -- is the same as
    >> [duplicate]: default
    @water{1} @&water{2}
    ```
- `auto scale` | `auto_scale`
  - `true`. All quantities have the implicit auto scale marker[^1] (`*`).
    ```cooklang
    >> [auto scale]: true
    @water{1}
    -- is the same as
    >> [auto scale]: false
    @water{1*}
    ```

    Note that ingredients with fixed scaling for each serving size[^1] are not
    affected by the auto scale mode.
  - `false` | `default`. The default cooklang behaviour.

## Temperature
Find temperatures in the text, without any markers. In the future this may be
extended to any unit.

For example, the temperature here will be parsed[^2] not as text, but as an inline
quantity.
```cooklang
Preheat the #oven to 180 ºC.
```

## Range values
Recipes are not always exact. This is a little improvement that should help
comunicating that in some cases.

```cooklang
@eggs{2-4}
@tomato sauce{200-300%ml}            -- works with units
@water{1.5-2%l}                      -- with decimal numbers too
@flour{100%g} ... @&flour{200-400%g} -- the total will be 300-500 g
```

[^1]: This is work in progress in `cooklang` but supported here.

[^2]: Currently this is done in the analysis pass. So in the AST there is no
concept of inline quantities.