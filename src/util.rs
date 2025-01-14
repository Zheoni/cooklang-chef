use std::borrow::Cow;

use anyhow::{bail, Context as _, Result};

use camino::Utf8Path;
use cooklang::{
    analysis::{CheckOptions, CheckResult},
    metadata::CooklangValueExt,
    Metadata, ScalableRecipe,
};
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

pub fn map_recipe(mut r: ScalableRecipe) -> ScalableRecipe {
    if let Some(emoji_str) = r
        .metadata
        .get("emoji")
        .and_then(|v| v.as_str())
        .and_then(get_emoji)
    {
        r.metadata.map.insert("emoji".into(), emoji_str.into());
    }
    r
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
            .and_then(|r| unwrap_recipe(r, self.file_name(), self.text()?.as_ref(), ctx))
    }

    pub fn parse_result(&self, ctx: &Context) -> Result<cooklang::RecipeResult> {
        let parser = ctx.parser()?;
        let options = match self {
            Input::File { entry, .. } => ctx.parse_options(Some(entry.path())),
            Input::Stdin { .. } => ctx.parse_options(None),
        };
        let r = parser
            .parse_with_options(self.text()?.as_ref(), options)
            .map(map_recipe);
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

    pub fn text(&self) -> Result<Cow<str>> {
        Ok(match self {
            Input::File { entry, .. } => entry.read()?.into_text().into(),
            Input::Stdin { text, .. } => text.as_str().into(),
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
        .and_then(|n| n.as_str())
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

    fn content(&self) -> Result<RecipeContent> {
        Ok(self.entry.read()?)
    }

    pub fn parsed(&self, ctx: &Context) -> Result<&cooklang::RecipeResult> {
        self.parsed
            .get_or_try_init(|| {
                let parser = ctx.parser()?;
                let r = self
                    .content()?
                    .parse_with_options(parser, ctx.parse_options(Some(self.entry.path())))
                    .map(map_recipe);
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
                    .metadata_with_options(parser, ctx.parse_options(None))
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

pub fn metadata_validator(
    key: &serde_yaml::Value,
    value: &serde_yaml::Value,
    opts: &mut CheckOptions,
) -> CheckResult {
    let Some(key) = key.as_str() else {
        opts.include(false);
        return CheckResult::Error(vec!["Metadata key is not a string".into()]);
    };

    match key {
        "tag" | "tags" => {
            if let Some(tags) = value.as_tags() {
                for t in tags {
                    let t = t.trim();
                    if t.is_empty() {
                        return CheckResult::Warning(vec!["The tag is empty".into()]);
                    } else if t.chars().count() > 32 {
                        return CheckResult::Warning(vec![TAG_TOO_LONG_MSG.into()]);
                    } else if !is_valid_tag(t) {
                        return CheckResult::Warning(vec![IS_VALID_TAG_MSG.into()]);
                    }
                }
            }
        }
        "emoji" => {
            if value.as_str().and_then(get_emoji).is_none() {
                opts.include(false);
                return CheckResult::Warning(vec!["Value is not an emoji".into()]);
            }
        }
        _ => {}
    }
    CheckResult::Ok
}

/// Checks that a tag is valid
///
/// A tag is valid when:
/// - 32 characters
/// - lowercase letters and numbers separated by a single '-'
/// - starts with a letter
pub fn is_valid_tag(tag: &str) -> bool {
    let tag_len = 1..=32;
    let re = regex!(r"^\p{Ll}[\p{Ll}\d]*(-[\p{Ll}\d]+)*$");

    tag_len.contains(&tag.chars().count()) && re.is_match(tag)
}

const IS_VALID_TAG_MSG: &str =
    "The tag should only have lower case letters and numbers separated by a single hyphen ('-')";

const TAG_TOO_LONG_MSG: &str = "The tag is too long";

pub fn get_emoji(s: &str) -> Option<&'static str> {
    if s.starts_with(':') && s.ends_with(':') {
        emojis::get_by_shortcode(&s[1..s.len() - 1]).map(|e| e.as_str())
    } else {
        emojis::get(s).map(|e| e.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_tag() {
        assert!(is_valid_tag("uwu"));
        assert!(is_valid_tag("italian-food"));
        assert!(is_valid_tag("contains-number-1"));
        assert!(is_valid_tag("unicode-ñçá"));
        assert!(!is_valid_tag(""));
        assert!(!is_valid_tag("1ow"));
        assert!(!is_valid_tag("111"));
        assert!(!is_valid_tag("1starts-with-number"));
        assert!(!is_valid_tag("many---hyphens"));
        assert!(!is_valid_tag("other/characters"));
        assert!(!is_valid_tag("other@[]chara€cters"));
    }
}
