//! Format a recipe as markdown

use std::{borrow::Cow, fmt::Write, io};

use cooklang::{
    convert::Converter,
    metadata::{IndexMap, Metadata, NameAndUrl, RecipeTime},
    model::{ComponentKind, Item, Section, Step},
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
        print_wrapped(&mut writer, desc)?;
        writeln!(writer)?;
    }

    ingredients(&mut writer, recipe, converter)?;
    cookware(&mut writer, recipe)?;
    sections(&mut writer, recipe)?;

    Ok(())
}

fn frontmatter(mut w: impl io::Write, metadata: &Metadata) -> Result<()> {
    #[derive(serde::Serialize)]
    struct CustomMetadata<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        slug: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        emoji: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        author: Option<&'a NameAndUrl<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<&'a NameAndUrl<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<&'a RecipeTime>,
        #[serde(skip_serializing_if = "Option::is_none")]
        servings: Option<&'a [u32]>,
        #[serde(borrow, flatten)]
        map: IndexMap<Cow<'a, str>, Cow<'a, str>>,
    }

    let map = CustomMetadata {
        slug: metadata.slug.as_deref(),
        emoji: metadata.slug.as_deref(),
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

    for ingredient in recipe
        .ingredients
        .iter()
        .filter(|i| !i.is_hidden() && !i.is_reference())
    {
        write!(w, "- ")?;
        if let Some(total_quantity) = ingredient
            .total_quantity(&recipe.ingredients, converter)
            .ok()
            .flatten()
        {
            write!(w, "*{total_quantity}* ")?;
        } else {
            let list = ingredient
                .all_quantities(&recipe.ingredients)
                .map(ToString::to_string)
                .reduce(|s, q| format!("{s}, {q}"));
            if let Some(list) = list {
                write!(w, "*{list}* ")?;
            }
        }

        write!(w, "{}", ingredient.display_name())?;

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
    for item in &recipe.cookware {
        write!(w, "- ")?;
        if let Some(value) = &item.quantity {
            write!(w, "*{value}* ")?;
        }
        writeln!(w, "{}", item.name)?;
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
    let mut step_counter = 0;
    for step in &section.steps {
        if !step.is_text {
            step_counter += 1;
            write!(w, "{}. ", step_counter)?;
        }
        w_step(w, step, recipe)?;
        writeln!(w)?;
    }
    Ok(())
}

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &ScaledRecipe) -> Result {
    let mut step_str = String::new();
    for item in &step.items {
        match item {
            Item::Text(t) => step_str.push_str(t),
            Item::Component(c) => {
                match c.kind {
                    ComponentKind::Ingredient => {
                        let igr = &recipe.ingredients[c.index];
                        step_str.push_str(igr.display_name().as_ref());
                    }
                    ComponentKind::Cookware => {
                        let cw = &recipe.cookware[c.index];
                        step_str.push_str(&cw.name);
                    }
                    ComponentKind::Timer => {
                        let t = &recipe.timers[c.index];
                        if let Some(name) = &t.name {
                            write!(&mut step_str, "({name})").unwrap();
                        }
                        write!(&mut step_str, "{}", t.quantity).unwrap();
                    }
                };
            }
            Item::InlineQuantity(index) => {
                let q = &recipe.inline_quantities[*index];
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
    static TERM_WIDTH: once_cell::sync::Lazy<usize> =
        once_cell::sync::Lazy::new(|| textwrap::termwidth().min(80));

    let lines = textwrap::wrap(text, textwrap::Options::new(*TERM_WIDTH));
    for line in lines {
        writeln!(w, "{}", line)?;
    }
    Ok(())
}
