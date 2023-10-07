//! Format a recipe as cooklang

use std::{fmt::Write, io};

use cooklang::{
    ast::{IntermediateData, Modifiers},
    metadata::Metadata,
    model::{Item, Section, Step},
    quantity::{Quantity, QuantityValue},
    IngredientReferenceTarget, Recipe,
};
use regex::Regex;

pub fn print_cooklang<D, V: QuantityValue>(
    recipe: &Recipe<D, V>,
    mut writer: impl io::Write,
) -> io::Result<()> {
    let w = &mut writer;

    metadata(w, &recipe.metadata)?;
    writeln!(w)?;
    sections(w, recipe)?;

    Ok(())
}

fn metadata(w: &mut impl io::Write, metadata: &Metadata) -> io::Result<()> {
    // TODO if the recipe has been scaled and multiple servings are defined
    // it can lead to the recipe not parsing.

    for (key, value) in &metadata.map {
        writeln!(w, ">> {key}: {value}")?;
    }
    Ok(())
}

fn sections<D, V: QuantityValue>(w: &mut impl io::Write, recipe: &Recipe<D, V>) -> io::Result<()> {
    for (index, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, index)?;
    }
    Ok(())
}

fn w_section<D, V: QuantityValue>(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &Recipe<D, V>,
    index: usize,
) -> io::Result<()> {
    if let Some(name) = &section.name {
        writeln!(w, "== {name} ==")?;
    } else if index > 0 {
        writeln!(w, "====")?;
    }
    for step in &section.steps {
        w_step(w, step, recipe)?;
        writeln!(w)?;
    }
    Ok(())
}

fn w_step<D, V: QuantityValue>(
    w: &mut impl io::Write,
    step: &Step,
    recipe: &Recipe<D, V>,
) -> io::Result<()> {
    let mut step_str = String::new();
    for item in &step.items {
        match item {
            Item::Text { value } => step_str.push_str(value),
            &Item::Ingredient { index } => {
                let igr = &recipe.ingredients[index];

                let intermediate_data = igr
                    .relation
                    .references_to()
                    .and_then(|(index, target)| calculate_intermediate_data(index, target));

                ComponentFormatter {
                    kind: ComponentKind::Ingredient,
                    modifiers: igr.modifiers(),
                    intermediate_data,
                    name: Some(&igr.name),
                    alias: igr.alias.as_deref(),
                    quantity: igr.quantity.as_ref(),
                    note: igr.note.as_deref(),
                }
                .format(&mut step_str)
            }
            &Item::Cookware { index } => {
                let cw = &recipe.cookware[index];
                ComponentFormatter {
                    kind: ComponentKind::Cookware,
                    modifiers: cw.modifiers(),
                    intermediate_data: None,
                    name: Some(&cw.name),
                    alias: cw.alias.as_deref(),
                    quantity: cw.quantity.clone().map(|v| Quantity::new(v, None)).as_ref(),
                    note: None,
                }
                .format(&mut step_str)
            }
            &Item::Timer { index } => {
                let t = &recipe.timers[index];
                ComponentFormatter {
                    kind: ComponentKind::Timer,
                    modifiers: Modifiers::empty(),
                    intermediate_data: None,
                    name: t.name.as_deref(),
                    alias: None,
                    quantity: t.quantity.as_ref(),
                    note: None,
                }
                .format(&mut step_str)
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
    let width = textwrap::termwidth().min(80);
    let mut options = textwrap::Options::new(width)
        .word_separator(textwrap::WordSeparator::Custom(component_word_separator));
    if step.is_text() {
        let indent = "> ";
        options = options.initial_indent(indent).subsequent_indent(indent);
    }
    let lines = textwrap::wrap(step_str.trim(), options);
    for line in lines {
        writeln!(w, "{}", line)?;
    }
    Ok(())
}

// This prevents spliting a multi word component in two lines, because that's
// invalid.
fn component_word_separator<'a>(
    line: &'a str,
) -> Box<dyn Iterator<Item = textwrap::core::Word<'a>> + 'a> {
    use textwrap::core::Word;

    let re = {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new(r"[@#~][^@#~]*\{[^\}]*\}").unwrap())
    };

    let mut words = vec![];
    let mut last_added = 0;
    let default_separator = textwrap::WordSeparator::new();

    for component in re.find_iter(line) {
        if last_added < component.start() {
            words.extend(default_separator.find_words(&line[last_added..component.start()]));
        }
        words.push(Word::from(&line[component.range()]));
        last_added = component.end();
    }
    if last_added < line.len() {
        words.extend(default_separator.find_words(&line[last_added..]));
    }
    Box::new(words.into_iter())
}

struct ComponentFormatter<'a, V: QuantityValue> {
    kind: ComponentKind,
    modifiers: Modifiers,
    intermediate_data: Option<IntermediateData>,
    name: Option<&'a str>,
    alias: Option<&'a str>,
    quantity: Option<&'a Quantity<V>>,
    note: Option<&'a str>,
}

enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

impl<'a, V: QuantityValue> ComponentFormatter<'a, V> {
    fn format(self, w: &mut String) {
        w.push(match self.kind {
            ComponentKind::Ingredient => '@',
            ComponentKind::Cookware => '#',
            ComponentKind::Timer => '~',
        });
        for m in self.modifiers {
            w.push(match m {
                Modifiers::RECIPE => '@',
                Modifiers::HIDDEN => '-',
                Modifiers::OPT => '?',
                Modifiers::REF => '&',
                Modifiers::NEW => '+',
                _ => panic!("Unknown modifier: {:?}", m),
            });
            if m == Modifiers::REF && self.intermediate_data.is_some() {
                use cooklang::ast::IntermediateRefMode::*;
                use cooklang::ast::IntermediateTargetKind::*;
                let IntermediateData {
                    ref_mode,
                    target_kind,
                    val,
                } = self.intermediate_data.unwrap();
                let repr = match (target_kind, ref_mode) {
                    (Step, Index) => format!("{val}"),
                    (Step, Relative) => format!("~{val}"),
                    (Section, Index) => format!("={val}"),
                    (Section, Relative) => format!("=~{val}"),
                };
                w.push_str(&format!("({repr})"));
            }
        }
        let mut multi_word = false;
        if let Some(name) = self.name {
            if name.chars().any(|c| !c.is_alphanumeric()) {
                multi_word = true;
            }
            w.push_str(name);
            if let Some(alias) = self.alias {
                multi_word = true;
                w.push('|');
                w.push_str(alias);
            }
        }
        if let Some(q) = self.quantity {
            w.push('{');
            w.push_str(&q.value.to_string());
            if let Some(unit) = q.unit_text() {
                write!(w, "%{}", unit).unwrap();
            }
            w.push('}');
        } else if multi_word {
            w.push_str("{}");
        }
        if let Some(note) = self.note {
            write!(w, "({note})").unwrap();
        }
    }
}

fn calculate_intermediate_data(
    index: usize,
    target: IngredientReferenceTarget,
) -> Option<IntermediateData> {
    use cooklang::ast::IntermediateRefMode::*;
    use cooklang::ast::IntermediateTargetKind::*;

    // TODO maybe use relative references for "close enough" references?
    let d = match target {
        IngredientReferenceTarget::Ingredient => return None,
        IngredientReferenceTarget::Step => IntermediateData {
            ref_mode: Index,
            target_kind: Step,
            val: index as i16,
        },
        IngredientReferenceTarget::Section => IntermediateData {
            ref_mode: Index,
            target_kind: Section,
            val: index as i16,
        },
    };

    Some(d)
}
