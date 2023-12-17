use anstream::println;
use anyhow::Result;
use clap::{builder::ArgPredicate, Args};
use cooklang::metadata::Metadata;
use cooklang_fs::{all_recipes, RecipeContent, RecipeEntry};

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
    #[arg(short, long)]
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
    let iter = all_recipes(&ctx.base_path, ctx.config.max_depth)?
        .map(CachedRecipeEntry::new)
        .filter(|entry| {
            if args.tag.is_empty() {
                return true;
            }

            let Ok(metadata) = entry.metadata(ctx, args.check) else {
                tracing::warn!(
                    "Skipping '{}': could not parse metadata",
                    entry.entry.path()
                );
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
                    let report = entry.parsed(ctx)?;
                    if report.report().has_errors() {
                        with_errors += 1;
                    }
                    if report.report().has_warnings() {
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
        let mut all = iter.collect::<Vec<_>>();
        all.sort_unstable_by(|a, b| a.path().cmp(b.path()));
        for entry in all {
            let row = list_row(ctx, &args, entry)?;
            table.add_row(row);
        }
        println!("{table}");
    }

    Ok(())
}

fn list_row(ctx: &Context, args: &ListArgs, entry: CachedRecipeEntry) -> Result<tabular::Row> {
    use owo_colors::OwoColorize;

    let mut row = tabular::Row::new();

    let name = if args.absolute_paths {
        entry.path().canonicalize()?.to_string_lossy().to_string()
    } else {
        let p = entry.path().strip_prefix(&ctx.base_path).unwrap();
        if args.paths {
            p.to_string()
        } else if let Some(parent) = entry
            .path()
            .parent()
            .and_then(|p| p.strip_prefix(&ctx.base_path).ok())
            .filter(|p| !p.as_str().is_empty())
        {
            format!(
                "{}{}{}",
                parent.cyan().italic(),
                std::path::MAIN_SEPARATOR.cyan(),
                entry.name()
            )
        } else {
            entry.name().to_string()
        }
    };
    row.add_ansi_cell(name);

    if args.tags {
        if let Ok(metadata) = entry.metadata(ctx, args.check) {
            if metadata.tags.is_empty() {
                row.add_ansi_cell(format!(" [{}]", "-".dimmed()));
            } else {
                row.add_cell(format!(" [{}]", metadata.tags.join(", ")));
            }
        } else {
            row.add_ansi_cell(format!(" ({})", "cannot parse".red().bold()));
        }
    } else {
        row.add_cell("");
    }

    if args.check {
        row.add_ansi_cell(format!(" [{}]", check_str(ctx, &entry)));
    } else {
        row.add_cell("");
    };

    if let Some(images) = args.images.then(|| entry.images().len()) {
        let s = if images > 0 {
            format!(" [{} image{}]", images, if images == 1 { "" } else { "s" })
        } else {
            format!(" [{}]", "no images".dimmed())
        };
        row.add_ansi_cell(s);
    } else {
        row.add_cell("");
    };

    Ok(row)
}

fn check_str(ctx: &Context, entry: &CachedRecipeEntry) -> String {
    use owo_colors::OwoColorize;

    entry
        .content()
        .ok()
        .and_then(|content| ctx.parse_content(content).ok())
        .map(|r| r.into_report())
        .map(|report| {
            if report.has_errors() {
                "Error".red().bold().to_string()
            } else if report.has_warnings() {
                "Warn".yellow().bold().to_string()
            } else {
                "Ok".green().bold().to_string()
            }
        })
        .unwrap_or("Could not check".red().dimmed().to_string())
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

    fn parsed(&self, ctx: &Context) -> Result<&cooklang::RecipeResult> {
        self.content()
            .and_then(|content| self.parsed.get_or_try_init(|| ctx.parse_content(content)))
            .map_err(anyhow::Error::from)
    }

    fn metadata(&self, ctx: &Context, try_full: bool) -> Result<&Metadata> {
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
                            .parsed(ctx)
                            .ok()
                            .and_then(|r| r.output())
                            .map(|r| &r.metadata)
                        {
                            return Ok(m.clone());
                        }
                    }
                    content
                        .metadata(ctx.parser()?)
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
