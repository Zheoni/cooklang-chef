use anyhow::{bail, Context as _, Result};

use camino::Utf8Path;
use cooklang::Metadata;
use cooklang_fs::{RecipeContent, RecipeEntry};

use crate::Context;

/// Utility to create lazy regex
/// from <https://docs.rs/once_cell/latest/once_cell/#lazily-compiled-regex>
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| {
            let _enter = tracing::trace_span!("regex", re = $re).entered();
            regex::Regex::new($re).unwrap()
        })
    }};
}
pub(crate) use regex;

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
        entry: cooklang_fs::RecipeEntry,
        override_name: Option<String>,
    },
    Stdin {
        text: String,
        name: Option<String>,
    },
}

impl Input {
    pub fn parse(&self, ctx: &Context) -> Result<cooklang::ScalableRecipe> {
        self.parse_result(ctx)
            .and_then(|r| unwrap_recipe(r, self.file_name(), self.text()?, ctx))
    }

    pub fn parse_result(&self, ctx: &Context) -> Result<cooklang::RecipeResult> {
        let parser = ctx.parser()?;
        let checker = match self {
            Input::File { entry, .. } => ctx.checker(Some(entry.path())),
            Input::Stdin { .. } => ctx.checker(None),
        };
        let r = parser.parse_with_recipe_ref_checker(self.text()?, checker);
        Ok(r)
    }

    pub fn name(&self) -> Result<&str> {
        let n = match self {
            Input::File {
                entry,
                override_name,
            } => override_name.as_deref().unwrap_or(entry.name()),
            Input::Stdin { name, .. } => name
                .as_deref()
                .ok_or(anyhow::anyhow!("No name given for recipe"))?,
        };
        Ok(n)
    }

    pub fn file_name(&self) -> &str {
        match &self {
            Input::File { entry: content, .. } => content.file_name(),
            Input::Stdin { name, .. } => name.as_deref().unwrap_or("STDIN"),
        }
    }

    pub fn text(&self) -> Result<&str> {
        Ok(match self {
            Input::File { entry, .. } => entry.read()?.text(),
            Input::Stdin { text, .. } => text,
        })
    }

    pub fn path(&self) -> Option<&Utf8Path> {
        match self {
            Input::File { entry: content, .. } => Some(content.path()),
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

pub fn meta_name(meta: &cooklang::Metadata) -> Option<&str> {
    ["name", "title"]
        .iter()
        .find_map(|&k| meta.map.get(k))
        .map(|n| n.as_str())
}

pub struct CachedRecipeEntry {
    entry: RecipeEntry,
    metadata: once_cell::unsync::OnceCell<Box<Metadata>>,
    parsed: once_cell::unsync::OnceCell<Box<cooklang::RecipeResult>>,
}

impl CachedRecipeEntry {
    pub fn new(entry: RecipeEntry) -> Self {
        Self {
            entry,
            metadata: Default::default(),
            parsed: Default::default(),
        }
    }

    pub fn content(&self) -> Result<&RecipeContent> {
        Ok(self.entry.read()?)
    }

    pub fn parsed(&self, ctx: &Context) -> Result<&cooklang::RecipeResult> {
        self.parsed
            .get_or_try_init(|| {
                let parser = ctx.parser()?;
                let checker = ctx.checker(Some(self.entry.path()));
                let r = self
                    .content()?
                    .parse_with_recipe_ref_checker(parser, checker);
                Ok(Box::new(r))
            })
            .map(|r| r.as_ref())
    }

    pub fn metadata(&self, ctx: &Context, try_full: bool) -> Result<&Metadata> {
        // first try cached full recipe
        if let Some(m) = self
            .parsed
            .get()
            .and_then(|r| r.output())
            .map(|r| &r.metadata)
        {
            return Ok(m);
        }

        self.metadata
            .get_or_try_init(|| {
                let parser = ctx.parser()?;
                if try_full {
                    if let Ok(r) = self.parsed(ctx) {
                        if let Some(m) = r.output().map(|r| &r.metadata) {
                            return Ok(Box::new(m.clone()));
                        }
                    }
                }
                let m = self
                    .content()?
                    .metadata(parser)
                    .into_output()
                    .ok_or(anyhow::anyhow!("Can't parse metadata"))?;
                Ok(Box::new(m))
            })
            .map(|m| m.as_ref())
    }
}

impl std::ops::Deref for CachedRecipeEntry {
    type Target = RecipeEntry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}
