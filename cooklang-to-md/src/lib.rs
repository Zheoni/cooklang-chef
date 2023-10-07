//! Format a recipe as markdown

use std::{fmt::Write, io};

use cooklang::{
    convert::Converter,
    metadata::{IndexMap, Metadata, NameAndUrl, RecipeTime},
    model::{Item, Section, Step},
    ScaledRecipe,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Error serializing YAML frontmatter")]
    Metadata(
        #[from]
        #[source]
        serde_yaml::Error,
    ),
}

pub type Result<T = ()> = std::result::Result<T, Error>;

pub fn print_md(
    recipe: &ScaledRecipe,
    converter: &Converter,
    mut writer: impl io::Write,
) -> Result {
    frontmatter(&mut writer, &recipe.metadata)?;

    writeln!(writer, "# {}", recipe.name)?;
    for tag in &recipe.metadata.tags {
        write!(writer, "#{tag} ")?;
    }
    if !recipe.metadata.tags.is_empty() {
        writeln!(writer)?;
    }
    writeln!(writer)?;

    if let Some(desc) = &recipe.metadata.description {
        print_wrapped_with_options(&mut writer, desc, |o| {
            o.initial_indent("> ").subsequent_indent("> ")
        })?;
        writeln!(writer)?;
    }

    ingredients(&mut writer, recipe, converter)?;
    cookware(&mut writer, recipe)?;
    sections(&mut writer, recipe)?;

    Ok(())
}

fn frontmatter(mut w: impl io::Write, metadata: &Metadata) -> Result<()> {
    if metadata.map.is_empty() {
        return Ok(());
    }

    #[derive(serde::Serialize)]
    struct CustomMetadata<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        emoji: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        author: Option<&'a NameAndUrl>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<&'a NameAndUrl>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<&'a RecipeTime>,
        #[serde(skip_serializing_if = "Option::is_none")]
        servings: Option<&'a [u32]>,
        #[serde(flatten)]
        map: IndexMap<String, String>,
    }

    let map = CustomMetadata {
        emoji: metadata.emoji.as_deref(),
        author: metadata.author.as_ref(),
        source: metadata.source.as_ref(),
        time: metadata.time.as_ref(),
        servings: metadata.servings.as_deref(),
        map: metadata.map_filtered(),
    };

    const FRONTMATTER_FENCE: &str = "---";
    writeln!(w, "{}", FRONTMATTER_FENCE)?;
    serde_yaml::to_writer(&mut w, &map)?;
    writeln!(w, "{}\n", FRONTMATTER_FENCE)?;
    Ok(())
}

fn ingredients(w: &mut impl io::Write, recipe: &ScaledRecipe, converter: &Converter) -> Result {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }

    writeln!(w, "## Ingredients")?;

    for entry in recipe.group_ingredients(converter) {
        let ingredient = entry.ingredient;

        if !ingredient.modifiers().should_be_listed() {
            continue;
        }

        write!(w, "- ")?;
        let q = match entry.quantity.total() {
            cooklang::quantity::TotalQuantity::None => None,
            cooklang::quantity::TotalQuantity::Single(q) => Some(q.to_string()),
            cooklang::quantity::TotalQuantity::Many(m) => m
                .into_iter()
                .map(|q| q.to_string())
                .reduce(|s, q| format!("{s}, {q}")),
        };
        if let Some(q) = q {
            write!(w, "*{q}* ")?;
        }

        write!(w, "{}", ingredient.display_name())?;

        if ingredient.modifiers().is_optional() {
            write!(w, " (optional)")?;
        }

        if let Some(note) = &ingredient.note {
            write!(w, " ({note})")?;
        }
        writeln!(w)?;
    }
    writeln!(w)?;

    Ok(())
}

fn cookware(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    if recipe.cookware.is_empty() {
        return Ok(());
    }

    writeln!(w, "## Cookware")?;
    for item in recipe
        .cookware
        .iter()
        .filter(|cw| cw.modifiers().should_be_listed())
    {
        write!(w, "- ")?;
        if let Some(value) = &item.quantity {
            write!(w, "*{value}* ")?;
        }
        write!(w, "{}", item.display_name())?;

        if item.modifiers().is_optional() {
            write!(w, " (optional)")?;
        }

        if let Some(note) = &item.note {
            write!(w, " ({note})")?;
        }
        writeln!(w)?;
    }

    writeln!(w)?;
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result<()> {
    writeln!(w, "## Steps")?;
    for (idx, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, idx + 1)?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &ScaledRecipe,
    idx: usize,
) -> Result {
    if section.name.is_some() || recipe.sections.len() > 1 {
        if let Some(name) = &section.name {
            writeln!(w, "### {name}")?;
        } else {
            writeln!(w, "### Section {idx}")?;
        }
    }
    for step in &section.steps {
        w_step(w, step, recipe)?;
        writeln!(w)?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &ScaledRecipe) -> Result {
    let mut step_str = String::new();

    if let Some(number) = step.number {
        write!(&mut step_str, "{}. ", number).unwrap();
    }

    for item in &step.items {
        match item {
            Item::Text { value } => step_str.push_str(value),
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                step_str.push_str(igr.display_name().as_ref());
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                step_str.push_str(&cw.name);
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                if let Some(name) = &t.name {
                    write!(&mut step_str, "({name})").unwrap();
                }
                if let Some(quantity) = &t.quantity {
                    write!(&mut step_str, "{}", quantity).unwrap();
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(&mut step_str, "{}", q.value).unwrap();
                if let Some(u) = q.unit_text() {
                    step_str.push_str(u);
                }
            }
        }
    }
    print_wrapped(w, &step_str)?;
    Ok(())
}

fn print_wrapped(w: &mut impl io::Write, text: &str) -> Result {
    print_wrapped_with_options(w, text, |o| o)
}

static TERM_WIDTH: once_cell::sync::Lazy<usize> =
    once_cell::sync::Lazy::new(|| textwrap::termwidth().min(80));

fn print_wrapped_with_options<F>(w: &mut impl io::Write, text: &str, f: F) -> Result
where
    F: FnOnce(textwrap::Options) -> textwrap::Options,
{
    let options = f(textwrap::Options::new(*TERM_WIDTH));
    let lines = textwrap::wrap(text, options);
    for line in lines {
        writeln!(w, "{}", line)?;
    }
    Ok(())
}
