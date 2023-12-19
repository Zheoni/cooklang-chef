use anyhow::{bail, Context as _, Result};

use camino::Utf8Path;

use crate::Context;

pub fn write_to_output<F>(output: Option<&Utf8Path>, f: F) -> Result<()>
where
    F: FnOnce(Box<dyn std::io::Write>) -> Result<()>,
{
    let stream: Box<dyn std::io::Write> = if let Some(path) = output {
        let file = std::fs::File::create(path).context("Failed to create output file")?;
        let stream = anstream::StripStream::new(file);
        Box::new(stream)
    } else {
        Box::new(anstream::stdout().lock())
    };
    f(stream)?;
    Ok(())
}

pub enum Input {
    File {
        content: cooklang_fs::RecipeContent,
        override_name: Option<String>,
    },
    Stdin {
        text: String,
        name: String,
    },
}

impl Input {
    pub fn parse(&self, ctx: &Context) -> Result<cooklang::ScalableRecipe> {
        self.parse_result(ctx)
            .and_then(|r| unwrap_recipe(r, self.name(), self.text(), ctx))
    }

    pub fn parse_result(&self, ctx: &Context) -> Result<cooklang::RecipeResult> {
        let parser = ctx.parser()?;
        let r = match self {
            Input::File { content, .. } => parser
                .parse_with_recipe_ref_checker(content.text(), ctx.checker(Some(content.path()))),
            Input::Stdin { text, .. } => {
                parser.parse_with_recipe_ref_checker(text, ctx.checker(None))
            }
        };
        Ok(r)
    }

    pub fn name(&self) -> &str {
        match self {
            Input::File {
                content,
                override_name,
            } => override_name.as_deref().unwrap_or(content.name()),
            Input::Stdin { name, .. } => name,
        }
    }

    pub fn file_name(&self) -> &str {
        match &self {
            Input::File { content, .. } => content.file_name(),
            Input::Stdin { name, .. } => name,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Input::File { content, .. } => content.text(),
            Input::Stdin { text, .. } => text,
        }
    }

    pub fn path(&self) -> Option<&Utf8Path> {
        match self {
            Input::File { content, .. } => Some(content.path()),
            Input::Stdin { .. } => None,
        }
    }
}

pub fn unwrap_recipe(
    r: cooklang::RecipeResult,
    file_name: &str,
    text: &str,
    ctx: &Context,
) -> Result<cooklang::ScalableRecipe> {
    if !r.is_valid() || ctx.global_args.warnings_as_errors && r.report().has_warnings() {
        let mut report = r.into_report();
        if ctx.global_args.ignore_warnings {
            report.remove_warnings();
        }
        report.eprint(file_name, text, ctx.color.color_stderr)?;
        bail!("Error parsing recipe");
    } else {
        let (recipe, warnings) = r.into_result().unwrap();
        if !ctx.global_args.ignore_warnings && !warnings.is_empty() {
            warnings.eprint(file_name, text, ctx.color.color_stderr)?;
        }
        Ok(recipe)
    }
}
