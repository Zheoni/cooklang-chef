//! Format a recipe for humans to read
//!
//! It uses colors controlled by `concolor` and `yansi`, so enable or
//! disable them at your own like.

use std::{collections::HashMap, io, time::Duration};

use cooklang::{
    convert::Converter,
    model::{Component, ComponentKind, Ingredient, IngredientListEntry, Item},
    quantity::Quantity,
    scale::ScaleOutcome,
    ScaledRecipe,
};
use std::fmt::Write;
use tabular::{Row, Table};
use yansi::Paint;

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
    steps(w, recipe)?;

    Ok(())
}

fn header(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    let title_text = format!(
        "  {}{}  ",
        Paint::masked(
            recipe
                .metadata
                .emoji
                .as_ref()
                .map(|s| format!("{s} "))
                .unwrap_or_default()
        ),
        recipe.name
    );
    write!(
        w,
        "{}",
        Paint::new(title_text)
            .bg(yansi::Color::Magenta)
            .fg(yansi::Color::White)
            .bold()
    )?;

    writeln!(w)?;
    if !recipe.metadata.tags.is_empty() {
        let mut tags = String::new();
        for tag in &recipe.metadata.tags {
            let hash = tag
                .chars()
                .enumerate()
                .map(|(i, c)| c as usize * i)
                .reduce(usize::wrapping_add)
                .map(|h| (h % 7))
                .unwrap_or_default();
            let color = match hash {
                0 => yansi::Color::Red,
                1 => yansi::Color::Blue,
                2 => yansi::Color::Cyan,
                3 => yansi::Color::Yellow,
                4 => yansi::Color::Green,
                5 => yansi::Color::Magenta,
                6 => yansi::Color::White,
                _ => unreachable!(),
            };
            tags += &Paint::new(format!("#{tag} ")).fg(color).to_string();
        }
        print_wrapped(w, &tags)?;
    }
    writeln!(w)
}

fn metadata(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    if let Some(desc) = &recipe.metadata.description {
        print_wrapped_with_options(w, desc, |o| {
            o.initial_indent("\u{2502} ").subsequent_indent("\u{2502}")
        })?;
        writeln!(w)?;
    }

    let some_meta = !recipe.metadata.map.is_empty();
    let mut meta_fmt =
        |name: &str, value: &str| writeln!(w, "{}: {}", Paint::green(name).bold(), value);
    if let Some(author) = &recipe.metadata.author {
        let text = author
            .name
            .as_deref()
            .or(author.url.as_ref().map(|u| u.as_str()))
            .unwrap_or("-");
        meta_fmt("author", text)?;
    }
    if let Some(source) = &recipe.metadata.source {
        let text = source
            .name
            .as_deref()
            .or(source.url.as_ref().map(|u| u.as_str()))
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
                    Paint::yellow(format!("[{s}]")).bold().to_string()
                } else {
                    s.to_string()
                }
            })
            .reduce(|a, b| format!("{a}|{b}"))
            .unwrap_or_default();
        if let Some(data) = recipe.scaled_data() {
            if data.target.index().is_none() {
                write!(
                    &mut text,
                    " {} {}",
                    Paint::red("->"),
                    Paint::red(data.target.target_servings()),
                )
                .unwrap();
            }
        }
        meta_fmt("servings", &text)?;
    }
    for (key, value) in recipe.metadata.map_filtered() {
        meta_fmt(&key, &value)?;
    }
    if some_meta {
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
    let list = recipe.ingredient_list(converter);
    for IngredientListEntry {
        index,
        quantity,
        outcome,
    } in list
    {
        let igr = &recipe.ingredients[index];
        if igr.is_hidden() {
            continue;
        }
        let mut is_fixed = false;
        let mut is_err = false;
        let s = outcome
            .map(|outcome| match outcome {
                ScaleOutcome::Fixed => {
                    there_is_fixed = true;
                    is_fixed = true;
                    yansi::Style::new(yansi::Color::Yellow)
                }
                ScaleOutcome::Error(_) => {
                    there_is_err = true;
                    is_err = true;
                    yansi::Style::default().bg(yansi::Color::Red)
                }
                ScaleOutcome::Scaled | ScaleOutcome::NoQuantity => yansi::Style::default(),
            })
            .unwrap_or_default();
        let mut row = Row::new()
            .with_cell(igr.display_name())
            .with_cell(if igr.is_optional() { "(optional)" } else { "" });
        match quantity.total() {
            cooklang::quantity::TotalQuantity::None => {
                row.add_cell("");
            }
            cooklang::quantity::TotalQuantity::Single(quantity) => {
                row.add_ansi_cell(s.paint(quantity_fmt(&quantity)));
            }
            cooklang::quantity::TotalQuantity::Many(list) => {
                let list = list
                    .into_iter()
                    .map(|q| quantity_fmt(&q))
                    .reduce(|s, q| format!("{s}, {q}"))
                    .unwrap();
                row.add_ansi_cell(s.wrap().paint(list));
            }
        }

        if let Some(note) = &igr.note {
            row.add_cell(format!("({note})"));
        } else {
            row.add_cell("");
        }
        table.add_row(row);
    }
    write!(w, "{table}")?;
    if there_is_fixed || there_is_err {
        if there_is_fixed {
            write!(w, "{} fixed value", Paint::yellow("\u{25A0}"))?;
        }
        if there_is_err {
            if there_is_fixed {
                write!(w, " | ")?;
            }
            write!(w, "{} error scaling", Paint::red("\u{25A0}"))?;
        }
        writeln!(w)?;
    }
    writeln!(w)
}

fn steps(w: &mut impl io::Write, recipe: &ScaledRecipe) -> Result {
    writeln!(w, "Steps:")?;
    for section in &recipe.sections {
        if let Some(name) = &section.name {
            writeln!(w, "{}", Paint::new(name).underline())?;
        }
        let mut step_counter = 0;
        for step in &section.steps {
            if step.is_text {
                let mut step_text = String::new();
                for item in &step.items {
                    if let Item::Text(text) = item {
                        step_text.push_str(text);
                    } else {
                        panic!("Not text in text step");
                    }
                }
                print_wrapped_with_options(w, &step_text, |o| o.initial_indent("  "))?;
            } else {
                step_counter += 1;
                write!(w, "{step_counter:>2}. ")?;

                let mut step_text = String::new();

                // contain all ingredients used in the step (the names), the vec
                // contains the exact indices used
                let mut step_igrs_dedup: HashMap<&str, Vec<usize>> = HashMap::new();
                for item in &step.items {
                    if let Item::Component(Component {
                        kind: ComponentKind::Ingredient,
                        index,
                    }) = item
                    {
                        let igr = &recipe.ingredients[*index];
                        step_igrs_dedup.entry(&igr.name).or_default().push(*index);
                    }
                }
                // only keep ingredients with quantities or a single ingredient
                // without if no one has a quantity
                for group in step_igrs_dedup.values_mut() {
                    let first = group.first().copied().unwrap();
                    group.retain(|&i| recipe.ingredients[i].quantity.is_some());
                    if group.is_empty() {
                        group.push(first);
                    }
                }

                // contains the ingreient and index (if any) in the line under
                // the step that shows the ingredients
                let mut step_igrs_line: Vec<(&Ingredient, Option<usize>)> = Vec::new();
                for item in &step.items {
                    match item {
                        Item::Text(text) => step_text += text,
                        Item::Component(c) => match c.kind {
                            ComponentKind::Ingredient => {
                                let igr = &recipe.ingredients[c.index];
                                write!(&mut step_text, "{}", Paint::green(igr.display_name()))
                                    .unwrap();
                                let pos = write_igr_count(
                                    &mut step_text,
                                    &step_igrs_dedup,
                                    c.index,
                                    &igr.name,
                                );
                                if step_igrs_dedup[igr.name.as_str()].contains(&c.index) {
                                    step_igrs_line.push((igr, pos));
                                }
                            }
                            ComponentKind::Cookware => {
                                let cookware = &recipe.cookware[c.index];
                                write!(&mut step_text, "{}", Paint::yellow(&cookware.name))
                                    .unwrap();
                            }
                            ComponentKind::Timer => {
                                let timer = &recipe.timers[c.index];
                                write!(
                                    &mut step_text,
                                    "{}",
                                    Paint::cyan(quantity_fmt(&timer.quantity))
                                )
                                .unwrap();
                            }
                        },
                        Item::InlineQuantity(index) => {
                            let q = &recipe.inline_quantities[*index];
                            write!(&mut step_text, "{}", Paint::red(quantity_fmt(q))).unwrap()
                        }
                    }
                }
                print_wrapped_with_options(w, &step_text, |o| o.subsequent_indent("    "))?;
                if step_igrs_line.is_empty() {
                    writeln!(w, "    [-]")?;
                } else {
                    let mut igrs_text = String::from("    [");
                    for (i, (igr, pos)) in step_igrs_line.iter().enumerate() {
                        write!(&mut igrs_text, "{}", igr.display_name()).unwrap();
                        if let Some(pos) = pos {
                            write_subscript(&mut igrs_text, &pos.to_string());
                        }
                        if igr.is_optional() {
                            write!(&mut igrs_text, " (opt)").unwrap();
                        }
                        if let Some(q) = &igr.quantity {
                            write!(&mut igrs_text, ": {}", Paint::new(quantity_fmt(q)).dimmed())
                                .unwrap();
                        }
                        if i != step_igrs_line.len() - 1 {
                            igrs_text += ", ";
                        }
                    }
                    igrs_text += "]";
                    print_wrapped_with_options(w, &igrs_text, |o| o.subsequent_indent("     "))?;
                }
            }
        }
        writeln!(w)?
    }
    Ok(())
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
        format!("{} {}", qty.value, Paint::new(unit.text()).italic())
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

fn print_wrapped_with_options<F>(w: &mut impl io::Write, text: &str, f: F) -> Result
where
    F: FnOnce(textwrap::Options) -> textwrap::Options,
{
    static TERM_WIDTH: once_cell::sync::Lazy<usize> =
        once_cell::sync::Lazy::new(|| textwrap::termwidth().min(80));

    let options = f(textwrap::Options::new(*TERM_WIDTH));
    let lines = textwrap::wrap(text, options);
    for line in lines {
        writeln!(w, "{}", line)?;
    }
    Ok(())
}
