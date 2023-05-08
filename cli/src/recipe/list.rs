use anyhow::Result;
use clap::{builder::ArgPredicate, Args};
use cooklang::{metadata::Metadata, CooklangParser};
use cooklang_fs::{all_recipes, RecipeContent, RecipeEntry};
use yansi::Paint;

use crate::Context;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Check recipes for correctness
    #[arg(short, long, default_value_if("long", ArgPredicate::IsPresent, "true"))]
    check: bool,

    /// Include images
    #[arg(short, long, default_value_if("long", ArgPredicate::IsPresent, "true"))]
    images: bool,

    /// Filter entries by tag
    #[arg(short, long)]
    tag: Vec<String>,

    /// Show tags in the list
    #[arg(short = 'T',
        long,
        default_value_ifs([
            ("long", ArgPredicate::IsPresent, "true"),
            ("tag", ArgPredicate::IsPresent, "true")
        ])
    )]
    tags: bool,

    /// Add `check` and `images` in one flag
    #[arg(short, long, conflicts_with_all = ["check", "images"])]
    long: bool,

    /// Display the relative path of the recipes
    #[arg(short, long)]
    paths: bool,

    /// Display the absolute path of the recipes
    #[arg(short = 'P', long, conflicts_with = "paths")]
    absolute_paths: bool,

    /// Only count the number of recipes
    #[arg(short = 'n', long, conflicts_with_all = ["paths", "absolute_paths"])]
    count: bool,
}

pub fn run(ctx: &Context, args: ListArgs) -> Result<()> {
    let iter = all_recipes(&ctx.base_dir, ctx.config.max_depth)
        .map(CachedRecipeEntry::new)
        .filter(|entry| {
            if args.tag.is_empty() {
                return true;
            }

            let parser = ctx
                .parser()
                .expect("Could not init parser when filtering by tags");
            let Ok(metadata) = entry.metadata(parser, args.check) else {
                tracing::warn!("Skipping '{}': could not parse metadata", entry.entry.path());
                return false;
            };

            args.tag.iter().all(|t| metadata.tags.contains(t))
        });
    if args.count {
        let mut count = 0;
        let mut with_warnings = 0;
        let mut with_errors = 0;
        let mut with_images = 0;
        let mut total_images = 0;
        for entry in iter {
            count += 1;
            if args.check || args.images {
                if args.check {
                    let content = entry.read()?;
                    let report = content.parse(ctx.parser()?).into_report();
                    if report.has_errors() {
                        with_errors += 1;
                    }
                    if report.has_warnings() {
                        with_warnings += 1;
                    }
                }
                if args.images {
                    let images = entry.images().len();
                    if images > 0 {
                        with_images += 1;
                    }
                    total_images += images;
                }
            }
        }

        use tabular::{row, table};
        let mut table = table!("{:>}  {:<}", row!("Recipes", count));
        if args.check {
            table.add_row(row!("With errors", with_errors));
            table.add_row(row!("With warnings", with_warnings));
        }
        if args.images {
            table.add_row(row!("With images", with_images));
            table.add_row(row!("Total images", total_images));
        }
        println!("{table}");
    } else {
        let mut table = tabular::Table::new("{:<}{:<}{:<}{:<}");
        for entry in iter {
            let row = list_row(ctx, &args, entry)?;
            table.add_row(row);
        }
        println!("{table}");
    }

    Ok(())
}

fn list_row(ctx: &Context, args: &ListArgs, entry: CachedRecipeEntry) -> Result<tabular::Row> {
    let mut row = tabular::Row::new();

    let name = if args.absolute_paths {
        entry.path().to_string()
    } else {
        let p = entry.path().strip_prefix(&ctx.base_dir).unwrap();
        if args.paths {
            p.to_string()
        } else if let Some(parent) = entry
            .path()
            .parent()
            .and_then(|p| p.strip_prefix(&ctx.base_dir).ok())
            .filter(|p| !p.as_str().is_empty())
        {
            format!(
                "{}{}{}",
                Paint::cyan(parent).italic(),
                Paint::cyan(std::path::MAIN_SEPARATOR),
                entry.name()
            )
        } else {
            entry.name().to_string()
        }
    };
    row.add_ansi_cell(name);

    if args.tags {
        if let Some(metadata) = ctx
            .parser()
            .ok()
            .and_then(|parser| entry.metadata(parser, args.check).ok())
        {
            if metadata.tags.is_empty() {
                row.add_ansi_cell(format!(" [{}]", Paint::new("-").dimmed()));
            } else {
                row.add_cell(format!(" [{}]", metadata.tags.join(", ")));
            }
        } else {
            row.add_ansi_cell(format!(" ({})", Paint::yellow("cannot parse")));
        }
    } else {
        row.add_cell("");
    }

    if args.check {
        row.add_ansi_cell(format!(" [{}]", check_str(ctx.parser()?, &entry)));
    } else {
        row.add_cell("");
    };

    if let Some(images) = args.images.then(|| entry.images().len()) {
        let s = if images > 0 {
            format!(" [{} image{}]", images, if images == 1 { "" } else { "s" })
        } else {
            format!(" [{}]", Paint::new("no images").dimmed())
        };
        row.add_ansi_cell(s);
    } else {
        row.add_cell("");
    };

    Ok(row)
}

fn check_str(parser: &CooklangParser, entry: &CachedRecipeEntry) -> Paint<&'static str> {
    entry
        .content()
        .ok()
        .map(|e| e.parse(parser).into_report())
        .map(|report| {
            if report.has_errors() {
                Paint::red("Error")
            } else if report.has_warnings() {
                Paint::yellow("Warn")
            } else {
                Paint::green("Ok")
            }
        })
        .unwrap_or(Paint::red("Could not check").dimmed())
}

struct CachedRecipeEntry {
    entry: RecipeEntry,
    content: once_cell::unsync::OnceCell<RecipeContent>,
    metadata: once_cell::unsync::OnceCell<Metadata>,
    parsed: once_cell::unsync::OnceCell<cooklang::RecipeResult>,
}

impl CachedRecipeEntry {
    fn new(entry: RecipeEntry) -> Self {
        Self {
            entry,
            content: Default::default(),
            metadata: Default::default(),
            parsed: Default::default(),
        }
    }

    fn content(&self) -> Result<&RecipeContent> {
        self.content
            .get_or_try_init(|| self.entry.read())
            .map_err(anyhow::Error::from)
    }

    fn parsed(&self, parser: &CooklangParser) -> Result<&cooklang::RecipeResult> {
        self.content()
            .map(|content| self.parsed.get_or_init(|| content.parse(parser)))
    }

    fn metadata(&self, parser: &CooklangParser, try_full: bool) -> Result<&Metadata> {
        match self
            .parsed
            .get()
            .and_then(|r| r.output())
            .map(|r| &r.metadata)
        {
            Some(m) => Ok(m),
            None => self.content().and_then(|content| {
                self.metadata.get_or_try_init(|| {
                    if try_full && self.parsed.get().is_none() {
                        if let Some(m) = self
                            .parsed(parser)
                            .ok()
                            .and_then(|r| r.output())
                            .map(|r| &r.metadata)
                        {
                            return Ok(m.clone());
                        }
                    }
                    content
                        .metadata(parser)
                        .take_output()
                        .ok_or(anyhow::anyhow!("Can't parse metadata"))
                })
            }),
        }
    }
}

impl std::ops::Deref for CachedRecipeEntry {
    type Target = RecipeEntry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}
