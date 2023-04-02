//! Format a recipe as cooklang

use std::{borrow::Cow, fmt::Write, io};

use cooklang::{
    ast::Modifiers,
    metadata::Metadata,
    model::{ComponentKind, Item, Section, Step},
    quantity::Quantity,
    ScaledRecipe,
};

pub fn print_cooklang(recipe: &ScaledRecipe, mut writer: impl io::Write) -> io::Result<()> {
    let w = &mut writer;

    metadata(w, &recipe.metadata)?;
    writeln!(w)?;
    sections(w, recipe)?;

    Ok(())
}

fn metadata(w: &mut impl io::Write, metadata: &Metadata) -> io::Result<()> {
    for (key, value) in &metadata.map {
        writeln!(w, ">> {key}: {value}")?;
    }
    Ok(())
}

fn sections(w: &mut impl io::Write, recipe: &ScaledRecipe) -> io::Result<()> {
    for (index, section) in recipe.sections.iter().enumerate() {
        w_section(w, section, recipe, index)?;
    }
    Ok(())
}

fn w_section(
    w: &mut impl io::Write,
    section: &Section,
    recipe: &ScaledRecipe,
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

fn w_step(w: &mut impl io::Write, step: &Step, recipe: &ScaledRecipe) -> io::Result<()> {
    let mut step_str = String::new();
    if step.is_text {
        step_str.push_str("> ");
    }
    for item in &step.items {
        match item {
            Item::Text(t) => step_str.push_str(t),
            Item::Component(c) => {
                match c.kind {
                    ComponentKind::Ingredient => {
                        let igr = &recipe.ingredients[c.index];
                        ComponentFormatter {
                            kind: c.kind,
                            modifiers: igr.modifiers(),
                            name: Some(&igr.name),
                            alias: igr.alias.as_ref(),
                            quantity: igr.quantity.as_ref(),
                            note: igr.note.as_ref(),
                        }
                        .format(&mut step_str)
                    }
                    ComponentKind::Cookware => {
                        let cw = &recipe.cookware[c.index];
                        ComponentFormatter {
                            kind: c.kind,
                            modifiers: Modifiers::empty(),
                            name: Some(&cw.name),
                            alias: None,
                            quantity: cw.quantity.clone().map(|v| Quantity::new(v, None)).as_ref(),
                            note: None,
                        }
                        .format(&mut step_str)
                    }
                    ComponentKind::Timer => {
                        let t = &recipe.timers[c.index];
                        ComponentFormatter {
                            kind: c.kind,
                            modifiers: Modifiers::empty(),
                            name: t.name.as_ref(),
                            alias: None,
                            quantity: Some(&t.quantity),
                            note: None,
                        }
                        .format(&mut step_str)
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
    let width = textwrap::termwidth().min(80);
    let lines = textwrap::wrap(&step_str, textwrap::Options::new(width));
    for line in lines {
        writeln!(w, "{}", line)?;
    }
    Ok(())
}

struct ComponentFormatter<'a> {
    kind: ComponentKind,
    modifiers: Modifiers,
    name: Option<&'a Cow<'a, str>>,
    alias: Option<&'a Cow<'a, str>>,
    quantity: Option<&'a Quantity<'a>>,
    note: Option<&'a Cow<'a, str>>,
}

impl<'a> ComponentFormatter<'a> {
    fn format(self, w: &mut String) {
        w.push(match self.kind {
            ComponentKind::Ingredient => '@',
            ComponentKind::Cookware => '#',
            ComponentKind::Timer => '~',
        });
        for m in self.modifiers {
            w.push(m.as_char());
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
            match &q.value {
                cooklang::quantity::QuantityValue::Fixed(v) => write!(w, "{v}").unwrap(),
                cooklang::quantity::QuantityValue::Scalable(v) => match v {
                    cooklang::quantity::ScalableValue::Linear(v) => write!(w, "{v}*").unwrap(),
                    cooklang::quantity::ScalableValue::ByServings(values) => {
                        write!(w, "{}", &values[0]).unwrap();
                        for v in &values[1..] {
                            write!(w, "|{v}").unwrap()
                        }
                    }
                },
            }
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
