//! Metadata of a recipe

use std::{borrow::Cow, ops::RangeInclusive};

pub use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Metadata<'a> {
    pub slug: Option<String>,
    pub description: Option<Cow<'a, str>>,
    pub tags: Vec<Cow<'a, str>>,
    pub emoji: Option<Cow<'a, str>>,
    pub author: Option<NameAndUrl<'a>>,
    pub source: Option<NameAndUrl<'a>>,
    pub time: Option<RecipeTime>,
    pub servings: Option<Vec<u32>>,
    #[serde(borrow)]
    pub map: IndexMap<Cow<'a, str>, Cow<'a, str>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameAndUrl<'a> {
    pub name: Option<Cow<'a, str>>,
    pub url: Option<Url>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(untagged, deny_unknown_fields)]
pub enum RecipeTime {
    Total(u32),
    Composed {
        #[serde(alias = "prep")]
        prep_time: Option<u32>,
        #[serde(alias = "cook")]
        cook_time: Option<u32>,
    },
}

impl<'a> Metadata<'a> {
    pub(crate) fn insert(
        &mut self,
        key: Cow<'a, str>,
        value: Cow<'a, str>,
    ) -> Result<(), MetadataError> {
        self.map.insert(key.clone(), value.clone());
        match key.as_ref() {
            "slug" => self.slug = Some(slugify(&value)),
            "description" => self.description = Some(value),
            "tag" | "tags" => {
                let new_tags = value
                    .split(',')
                    .map(|s| Cow::from(s.trim().to_owned()))
                    .collect::<Vec<_>>();
                if new_tags.iter().any(|t| !is_valid_tag(t)) {
                    return Err(MetadataError::InvalidTag {
                        tag: value.into_owned(),
                    });
                }
                self.tags.extend(new_tags);
            }
            "emoji" => {
                if emojis::get(&value).is_some() {
                    self.emoji = Some(value);
                } else {
                    return Err(MetadataError::NotEmoji {
                        value: value.to_string(),
                    });
                }
            }
            "author" => self.author = Some(NameAndUrl::new(value)),
            "source" => self.source = Some(NameAndUrl::new(value)),
            "time" => self.time = Some(RecipeTime::Total(parse_time(&value)?)),
            "prep_time" | "prep time" => {
                let cook_time = self.time.and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { cook_time, .. } => cook_time,
                });
                self.time = Some(RecipeTime::Composed {
                    prep_time: Some(parse_time(&value)?),
                    cook_time,
                });
            }
            "cook_time" | "cook time" => {
                let prep_time = self.time.and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { prep_time, .. } => prep_time,
                });
                self.time = Some(RecipeTime::Composed {
                    prep_time,
                    cook_time: Some(parse_time(&value)?),
                });
            }
            "servings" => {
                self.servings = Some(
                    value
                        .split('|')
                        .map(str::trim)
                        .map(str::parse)
                        .collect::<Result<_, _>>()?,
                )
            }
            _ => {}
        }

        Ok(())
    }

    /// Returns a copy of [Self::map] but with all "special" metadata values
    /// removed
    pub fn map_filtered(&self) -> IndexMap<Cow<str>, Cow<str>> {
        const ALL_KNOWN_KEYS: &[&str] = &[
            "slug",
            "description",
            "tag",
            "tags",
            "emoji",
            "author",
            "source",
            "time",
            "prep_time",
            "prep time",
            "cook_time",
            "cook time",
            "servings",
        ];
        let mut new_map = self.map.clone();
        new_map.retain(|key, _| !ALL_KNOWN_KEYS.contains(&key.as_ref()));
        new_map
    }
}

/// Returns minutes
fn parse_time(s: &str) -> Result<u32, std::num::ParseIntError> {
    match humantime::parse_duration(s) {
        Ok(duration) => Ok((duration.as_secs_f64() / 60.0).round() as u32),
        Err(_) => s.parse(),
    }
}

impl<'a> NameAndUrl<'a> {
    pub fn new(s: Cow<'a, str>) -> Self {
        let re = regex!(r"^(\w+(?:\s\w+)*)\s+<([^>]+)>$");
        if let Some(captures) = re.captures(&s) {
            let name = &captures.get(1).unwrap();
            if let Ok(url) = Url::parse(captures[2].trim()) {
                return NameAndUrl {
                    name: Some(match &s {
                        Cow::Borrowed(s) => s[name.range()].into(),
                        Cow::Owned(s) => s[name.range()].to_owned().into(),
                    }),
                    url: Some(url),
                };
            }
        }

        if let Ok(url) = Url::parse(&s) {
            NameAndUrl {
                name: None,
                url: Some(url),
            }
        } else {
            NameAndUrl {
                name: Some(s),
                url: None,
            }
        }
    }
}

impl RecipeTime {
    pub fn total(self) -> u32 {
        match self {
            RecipeTime::Total(t) => t,
            RecipeTime::Composed {
                prep_time,
                cook_time,
            } => prep_time.iter().chain(cook_time.iter()).sum(),
        }
    }
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("Value is not an emoji: {value}")]
    NotEmoji { value: String },
    #[error("Invalid tag: {tag}")]
    InvalidTag { tag: String },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

const TAG_LEN: RangeInclusive<usize> = 3..=32;
fn is_valid_tag(tag: &str) -> bool {
    let re = regex!(r"^\p{Ll}[\p{Ll}\d]*(-[\p{Ll}\d]+)*$");

    TAG_LEN.contains(&tag.chars().count()) && re.is_match(tag)
}

pub fn slugify(text: &str) -> String {
    let text = text
        .trim()
        .replace(|c: char| (c.is_whitespace() || c == '_'), "-")
        .replace(|c: char| !(c.is_alphanumeric() || c == '-'), "")
        .trim_matches('-')
        .to_lowercase();

    let slug = regex!(r"--+").replace_all(&text, "-");

    slug.into()
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
        assert!(!is_valid_tag("ow"));
        assert!(!is_valid_tag("1ow"));
        assert!(!is_valid_tag("111"));
        assert!(!is_valid_tag("1starts-with-number"));
        assert!(!is_valid_tag("many---hyphens"));
        assert!(!is_valid_tag("other/characters"));
        assert!(!is_valid_tag("other@[]chara€cters"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("text"), "text");
        assert_eq!(slugify("text with spaces"), "text-with-spaces");
        assert_eq!(
            slugify("text with      many\tspaces"),
            "text-with-many-spaces"
        );
        assert_eq!(slugify("text with CAPS"), "text-with-caps");
        assert_eq!(slugify("text with CAPS"), "text-with-caps");
        assert_eq!(slugify("text_with_underscores"), "text-with-underscores");
        assert_eq!(slugify("WhATever_--thiS - - is"), "whatever-this-is");
        assert_eq!(slugify("Sensible recipe name"), "sensible-recipe-name");
    }
}
