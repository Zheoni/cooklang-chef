//! Format a recipe for humans to read
//!
//! This will always write ansi colours. Use something like
//! [`anstream`](https://docs.rs/anstream) to remove them if needed.

use std::{collections::HashMap, io, time::Duration};

use cooklang::{
    convert::Converter,
    ingredient_list::GroupedIngredient,
    model::{Ingredient, IngredientReferenceTarget, Item},
    quantity::Quantity,
    scale::ScaleOutcome,
    ScaledRecipe, Section, Step,
};
use owo_colors::OwoColorize;
use std::fmt::Write;
use tabular::{Row, Table};

mod style;
use style::styles;
pub use style::{set_styles, CookStyles};

pub type Result<T = ()> = std::result::Result<T, io::Error>;

pub fn print_human(
    recipe: &ScaledRecipe,
    converter: &Converter,
    mut writer: impl std::io::Write,
) -> Result {
    let w = &mut writer;

    header(w, recipe)?;
    metadata(w, recipe)?;
    ingredients(w, recipe, converter)?;
    cookware(w, recipe)?;
    steps(w, recipe)?;

    Ok(())
}

fn header(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    let title_text = format!(
        " {}{} ",
        recipe
            .metadata
            .emoji
            .as_ref()
            .map(|s| format!("{s} "))
            .unwrap_or_default(),
        recipe.name
    );
    writeln!(w, "{}", title_text.style(styles().title))?;
    if !recipe.metadata.tags.is_empty() {
        let mut tags = String::new();
        for tag in &recipe.metadata.tags {
            let color = tag_color(tag);
            write!(&mut tags, "{} ", format!("#{tag}").color(color)).unwrap();
        }
        print_wrapped(w, &tags)?;
    }
    writeln!(w)
}

fn tag_color(tag: &str) -> owo_colors::AnsiColors {
    let hash = tag
        .chars()
        .enumerate()
        .map(|(i, c)| c as usize * i)
        .reduce(usize::wrapping_add)
        .map(|h| (h % 7))
        .unwrap_or_default();
    match hash {
        0 => owo_colors::AnsiColors::Red,
        1 => owo_colors::AnsiColors::Blue,
        2 => owo_colors::AnsiColors::Cyan,
        3 => owo_colors::AnsiColors::Yellow,
        4 => owo_colors::AnsiColors::Green,
        5 => owo_colors::AnsiColors::Magenta,
        6 => owo_colors::AnsiColors::White,
        _ => unreachable!(),
    }
}

fn metadata(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    if let Some(desc) = &recipe.metadata.description {
        print_wrapped_with_options(w, desc, |o| {
            o.initial_indent("\u{2502} ").subsequent_indent("\u{2502}")
        })?;
        writeln!(w)?;
    }

    let mut meta_fmt =
        |name: &str, value: &str| writeln!(w, "{}: {}", name.style(styles().meta_key), value);
    if let Some(author) = &recipe.metadata.author {
        let text = author
            .name()
            .or(author.url().map(|u| u.as_str()))
            .unwrap_or("-");
        meta_fmt("author", text)?;
    }
    if let Some(source) = &recipe.metadata.source {
        let text = source
            .name()
            .or(source.url().map(|u| u.as_str()))
            .unwrap_or("-");
        meta_fmt("source", text)?;
    }
    if let Some(time) = &recipe.metadata.time {
        let time_fmt = |t: u32| {
            format!(
                "{}",
                humantime::format_duration(Duration::from_secs(t as u64 * 60))
            )
        };
        match time {
            cooklang::metadata::RecipeTime::Total(t) => meta_fmt("time", &time_fmt(*t))?,
            cooklang::metadata::RecipeTime::Composed {
                prep_time,
                cook_time,
            } => {
                if let Some(p) = prep_time {
                    meta_fmt("prep time", &time_fmt(*p))?
                }
                if let Some(c) = cook_time {
                    meta_fmt("cook time", &time_fmt(*c))?;
                }
                meta_fmt("total time", &time_fmt(time.total()))?;
            }
        }
    }
    if let Some(servings) = &recipe.metadata.servings {
        let index = recipe
            .scaled_data()
            .and_then(|d| d.target.index())
            .or_else(|| recipe.is_default_scaled().then_some(0));
        let mut text = servings
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if Some(i) == index {
                    format!("[{s}]")
                        .style(styles().selected_servings)
                        .to_string()
                } else {
                    s.to_string()
                }
            })
            .reduce(|a, b| format!("{a}|{b}"))
            .unwrap_or_default();
        if let Some(data) = recipe.scaled_data() {
            if data.target.index().is_none() {
                text = format!(
                    "{} {} {}",
                    text.strikethrough().dimmed(),
                    "\u{2192}".red(),
                    data.target.target_servings().red()
                );
            }
        }
        meta_fmt("servings", &text)?;
    }
    for (key, value) in recipe.metadata.map_filtered() {
        meta_fmt(&key, &value)?;
    }
    if !recipe.metadata.map.is_empty() {
        writeln!(w)?;
    }
    Ok(())
}

fn ingredients(w: &mut impl io::Write, recipe: &ScaledRecipe, converter: &Converter) -> Result {
    if recipe.ingredients.is_empty() {
        return Ok(());
    }
    writeln!(w, "Ingredients:")?;
    let mut table = Table::new("  {:<} {:<}    {:<} {:<}");
    let mut there_is_fixed = false;
    let mut there_is_err = false;
    let trinagle = " \u{26a0}";
    let octagon = " \u{2BC3}";
    for entry in recipe.group_ingredients(converter) {
        let GroupedIngredient {
            ingredient: igr,
            quantity,
            outcome,
            ..
        } = entry;
        if !igr.modifiers().should_be_listed() {
            continue;
        }
        let mut is_fixed = false;
        let mut is_err = false;
        let (outcome_style, outcome_char) = outcome
            .map(|outcome| match outcome {
                ScaleOutcome::Fixed => {
                    there_is_fixed = true;
                    is_fixed = true;
                    (owo_colors::Style::new().yellow(), trinagle)
                }
                ScaleOutcome::Error(_) => {
                    there_is_err = true;
                    is_err = true;
                    (owo_colors::Style::new().red(), octagon)
                }
                ScaleOutcome::Scaled | ScaleOutcome::NoQuantity => (owo_colors::Style::new(), ""),
            })
            .unwrap_or_default();
        let mut row = Row::new().with_cell(igr.display_name());
        if igr.modifiers().is_optional() {
            row.add_ansi_cell("(optional)".style(styles().opt_marker));
        } else {
            row.add_cell("");
        }
        let content = match quantity.total() {
            cooklang::quantity::TotalQuantity::None => "".to_string(),
            cooklang::quantity::TotalQuantity::Single(quantity) => {
                quantity_fmt(&quantity).style(outcome_style).to_string()
            }
            cooklang::quantity::TotalQuantity::Many(list) => list
                .into_iter()
                .map(|q| quantity_fmt(&q).style(outcome_style).to_string())
                .reduce(|s, q| format!("{s}, {q}"))
                .unwrap(),
        };
        row.add_ansi_cell(format!("{content}{}", outcome_char.style(outcome_style)));

        if let Some(note) = &igr.note {
            row.add_cell(format!("({note})"));
        } else {
            row.add_cell("");
        }
        table.add_row(row);
    }
    write!(w, "{table}")?;
    if there_is_fixed || there_is_err {
        writeln!(w)?;
        if there_is_fixed {
            write!(w, "{} {}", trinagle.trim().yellow(), "fixed value".yellow())?;
        }
        if there_is_err {
            if there_is_fixed {
                write!(w, " | ")?;
            }
            write!(w, "{} {}", octagon.trim().red(), "error scaling".red())?;
        }
        writeln!(w)?;
    }
    writeln!(w)
}

fn cookware(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    if recipe.cookware.is_empty() {
        return Ok(());
    }
    writeln!(w, "Cookware:")?;
    let mut table = Table::new("  {:<} {:<}    {:<} {:<}");
    for item in recipe
        .cookware
        .iter()
        .filter(|cw| cw.modifiers().should_be_listed())
    {
        let mut row = Row::new().with_cell(item.display_name()).with_cell(
            if item.modifiers().is_optional() {
                "(optional)"
            } else {
                ""
            },
        );

        if let Some(q) = &item.quantity {
            row.add_cell(q.to_string());
        } else {
            row.add_cell("");
        }

        if let Some(note) = &item.note {
            row.add_cell(format!("({note})"));
        } else {
            row.add_cell("");
        }

        table.add_row(row);
    }
    writeln!(w, "{table}")?;
    Ok(())
}

fn steps(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    writeln!(w, "Steps:")?;
    for (section_index, section) in recipe.sections.iter().enumerate() {
        if recipe.sections.len() > 1 {
            writeln!(
                w,
                "{: ^width$}",
                format!("─── § {} ───", section_index + 1),
                width = TERM_WIDTH
            )?;
        }

        if let Some(name) = &section.name {
            writeln!(w, "{}:", name.style(styles().section_name))?;
        }

        for step in &section.steps {
            let (step_text, step_ingredients) = step_text(recipe, section, step);
            if step.is_text() {
                print_wrapped_with_options(w, step_text.trim(), |o| o.initial_indent(" "))?;
                writeln!(w)?;
            } else {
                write!(w, "{:>2}. ", step.number.unwrap())?;
                print_wrapped_with_options(w, step_text.trim(), |o| o.subsequent_indent("    "))?;
            }
            if let Some(step_ingredients) = step_ingredients {
                print_wrapped_with_options(w, &step_ingredients, |o| {
                    let indent = "     "; // 5
                    o.initial_indent(indent).subsequent_indent(indent)
                })?;
            }
        }
    }
    Ok(())
}

fn step_text(recipe: &ScaledRecipe, section: &Section, step: &Step) -> (String, Option<String>) {
    if step.is_text() {
        let mut step_text = String::new();
        for item in &step.items {
            if let Item::Text { value } = item {
                step_text.push_str(value);
            } else {
                panic!("Not text in text step");
            }
        }
        return (step_text, None);
    }
    let mut step_text = String::new();

    let step_igrs_dedup = build_step_igrs_dedup(step, recipe);

    // contains the ingredient and index (if any) in the line under
    // the step that shows the ingredients
    let mut step_igrs_line: Vec<(&Ingredient, Option<usize>)> = Vec::new();

    for item in &step.items {
        match item {
            Item::Text { value } => step_text += value,
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];
                write!(
                    &mut step_text,
                    "{}",
                    igr.display_name().style(styles().ingredient)
                )
                .unwrap();
                let pos = write_igr_count(&mut step_text, &step_igrs_dedup, index, &igr.name);
                if step_igrs_dedup[igr.name.as_str()].contains(&index) {
                    step_igrs_line.push((igr, pos));
                }
            }
            &Item::Cookware { index } => {
                let cookware = &recipe.cookware[index];
                write!(&mut step_text, "{}", cookware.name.style(styles().cookware)).unwrap();
            }
            &Item::Timer { index } => {
                let timer = &recipe.timers[index];

                match (&timer.quantity, &timer.name) {
                    (Some(quantity), Some(name)) => {
                        let s = format!(
                            "{} ({})",
                            quantity_fmt(quantity).style(styles().timer),
                            name.style(styles().timer),
                        );
                        write!(&mut step_text, "{}", s).unwrap();
                    }
                    (Some(quantity), None) => {
                        write!(
                            &mut step_text,
                            "{}",
                            quantity_fmt(quantity).style(styles().timer)
                        )
                        .unwrap();
                    }
                    (None, Some(name)) => {
                        write!(&mut step_text, "{}", name.style(styles().timer)).unwrap();
                    }
                    (None, None) => unreachable!(), // guaranteed in parsing
                }
            }
            &Item::InlineQuantity { index } => {
                let q = &recipe.inline_quantities[index];
                write!(
                    &mut step_text,
                    "{}",
                    quantity_fmt(q).style(styles().inline_quantity)
                )
                .unwrap()
            }
        }
    }

    // This is only for the line where ingredients are placed

    if step_igrs_line.is_empty() {
        return (step_text, Some("[-]".into()));
    }
    let mut igrs_text = String::from("[");
    for (i, (igr, pos)) in step_igrs_line.iter().enumerate() {
        write!(&mut igrs_text, "{}", igr.display_name()).unwrap();
        if let Some(pos) = pos {
            write_subscript(&mut igrs_text, &pos.to_string());
        }
        if igr.modifiers().is_optional() {
            write!(&mut igrs_text, "{}", " (opt)".style(styles().opt_marker)).unwrap();
        }
        if let Some(source) = inter_ref_text(igr, section) {
            write!(
                &mut igrs_text,
                "{}",
                format!(" from {source}").style(styles().intermediate_ref)
            )
            .unwrap();
        }
        if let Some(q) = &igr.quantity {
            write!(
                &mut igrs_text,
                ": {}",
                quantity_fmt(q).style(styles().step_igr_quantity)
            )
            .unwrap();
        }
        if i != step_igrs_line.len() - 1 {
            igrs_text += ", ";
        }
    }
    igrs_text += "]";
    (step_text, Some(igrs_text))
}

fn inter_ref_text(igr: &Ingredient, section: &Section) -> Option<String> {
    match igr.relation.references_to() {
        Some((target_sect, IngredientReferenceTarget::Section)) => {
            Some(format!("section {}", target_sect + 1))
        }
        Some((target_step, IngredientReferenceTarget::Step)) => {
            let step = &section.steps[target_step];
            let s = if let Some(target_number) = step.number {
                format!("step {target_number}")
            } else {
                let from = section.steps[..target_step]
                    .iter()
                    .rev()
                    .find_map(|s| s.number);
                let to = section.steps[target_step..].iter().find_map(|s| s.number);
                match (from, to) {
                    (Some(from), Some(to)) => {
                        format!("between steps {from}-{to}")
                    }
                    // It means that the text step is the first one or
                    // that all steps before this are also text steps
                    (None, Some(_to)) => "text before steps".to_string(),
                    // for this to happen, the text step should be the last
                    // in the section. Because a text step cannot contain
                    // references, and a reference is always to past steps,
                    // this cannot occur.
                    (Some(_from), None) => {
                        panic!("impossible intermediate reference")
                    }
                    (None, None) => "text before".to_string(),
                }
            };
            Some(s)
        }
        _ => None,
    }
}

fn build_step_igrs_dedup<'a>(
    step: &'a Step,
    recipe: &'a ScaledRecipe,
) -> HashMap<&'a str, Vec<usize>> {
    // contain all ingredients used in the step (the names), the vec
    // contains the exact indices used
    let mut step_igrs_dedup: HashMap<&str, Vec<usize>> = HashMap::new();
    for item in &step.items {
        if let Item::Ingredient { index } = item {
            let igr = &recipe.ingredients[*index];
            step_igrs_dedup.entry(&igr.name).or_default().push(*index);
        }
    }

    // for each name only keep entries that provide information:
    // - if it has a quantity
    // - if it's an intermediate reference
    // - at least one if it's empty
    for group in step_igrs_dedup.values_mut() {
        let first = group.first().copied().unwrap();
        group.retain(|&i| {
            let igr = &recipe.ingredients[i];
            igr.quantity.is_some() || igr.relation.is_intermediate_reference()
        });
        if group.is_empty() {
            group.push(first);
        }
    }
    step_igrs_dedup
}

fn write_igr_count(
    buffer: &mut String,
    step_igrs: &HashMap<&str, Vec<usize>>,
    index: usize,
    name: &str,
) -> Option<usize> {
    let entries = &step_igrs[name];
    if entries.len() <= 1 {
        return None;
    }
    if let Some(mut pos) = entries.iter().position(|&i| i == index) {
        pos += 1;
        write_subscript(buffer, &pos.to_string());
        Some(pos)
    } else {
        None
    }
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value, unit.text().italic())
    } else {
        format!("{}", qty.value)
    }
}

fn write_subscript(buffer: &mut String, s: &str) {
    buffer.reserve(s.len());
    s.chars()
        .map(|c| match c {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            '9' => '₉',
            _ => c,
        })
        .for_each(|c| buffer.push(c))
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
