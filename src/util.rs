use anyhow::{bail, Context as _, Result};

use camino::Utf8Path;
use cooklang_fs::RecipeContent;

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
        content: RecipeContent,
        override_name: Option<String>,
    },
    Stdin {
        text: String,
        recipe_name: String,
    },
}

impl Input {
    pub fn parse(&self, ctx: &Context) -> Result<cooklang::Recipe> {
        match self {
            Input::File {
                content,
                override_name,
            } => {
                let r = ctx.parse_content(content)?.map(|mut r| {
                    if let Some(name) = override_name {
                        r.name = name.clone();
                    }
                    r
                });
                unwrap_recipe(r, content.file_name(), content.text(), ctx)
            }
            Input::Stdin { text, recipe_name } => {
                let r = ctx.parser()?.parse_with_recipe_ref_checker(
                    text,
                    recipe_name,
                    ctx.checker(None),
                );
                unwrap_recipe(r, recipe_name, text, ctx)
            }
        }
    }

    pub fn path(&self) -> Option<&Utf8Path> {
        match self {
            Input::File { content, .. } => Some(content.path()),
            Input::Stdin { .. } => None,
        }
    }
}

fn unwrap_recipe(
    r: cooklang::RecipeResult,
    file_name: &str,
    text: &str,
    ctx: &Context,
) -> Result<cooklang::Recipe> {
    if r.invalid() || ctx.global_args.warnings_as_errors && r.has_warnings() {
        r.into_report().eprint(
            file_name,
            text,
            ctx.global_args.ignore_warnings,
            ctx.color.color_stderr,
        )?;
        bail!("Error parsing recipe");
    } else {
        let (recipe, warnings) = r.into_result().unwrap();
        if !ctx.global_args.ignore_warnings && !warnings.is_empty() {
            warnings.eprint(file_name, text, ctx.color.color_stderr)?;
        }
        Ok(recipe)
    }
}
