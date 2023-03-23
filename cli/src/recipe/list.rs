use anyhow::Result;
use clap::{builder::ArgPredicate, Args};
use cooklang::CooklangParser;
use cooklang_fs::{DirEntry, FsIndex};
use termtree::Tree;
use yansi::Paint;

use crate::Context;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Show the output as a tree
    #[arg(short, long)]
    tree: bool,

    /// Check recipes for correctness
    #[arg(short, long, default_value_if("long", ArgPredicate::IsPresent, "true"))]
    check: bool,

    /// Include images
    #[arg(short, long, default_value_if("long", ArgPredicate::IsPresent, "true"))]
    images: bool,

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
    #[arg(short = 'n', long, conflicts_with_all = ["tree", "paths", "absolute_paths"])]
    count: bool,
}

pub fn run(ctx: &Context, args: ListArgs) -> Result<()> {
    let iter = FsIndex::all(&ctx.base_dir, ctx.config.max_depth);
    if args.tree {
        let mut iter = iter.peekable();
        let root = iter.next().unwrap();
        let tree = build_tree(&mut iter, root, ctx, &args)?;
        // dont print base dir
        for l in tree.leaves {
            print!("{l}");
        }
    } else if args.count {
        let mut count = 0;
        let mut with_warnings = 0;
        let mut with_errors = 0;
        let mut with_images = 0;
        let mut total_images = 0;
        for entry in iter.filter(|e| e.file_type().is_file()) {
            count += 1;
            if args.check || args.images {
                let entry = cooklang_fs::RecipeEntry::try_from(entry)
                    .expect("not recipe returned from iterator");

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
        // dont print dirs
        let mut table = tabular::Table::new("{:<} {:<} {:<}");
        for entry in iter.filter(|e| e.file_type().is_file()) {
            let (name, check, images) = list_label(ctx, &args, entry)?;
            table.add_row(
                tabular::Row::new()
                    .with_ansi_cell(name)
                    .with_ansi_cell(check)
                    .with_ansi_cell(images),
            );
        }
        println!("{table}");
    }

    Ok(())
}

fn build_tree(
    iter: &mut std::iter::Peekable<impl Iterator<Item = cooklang_fs::DirEntry>>,
    entry: cooklang_fs::DirEntry,
    ctx: &Context,
    args: &ListArgs,
) -> Result<Tree<String>> {
    let mut root = Tree::new(tree_label(ctx, args, &entry)?);
    if entry.file_type().is_dir() {
        while let Some(inner_entry) = iter.peek() {
            if inner_entry.depth() <= entry.depth() {
                break;
            }
            let inner_entry = iter.next().unwrap();
            root.push(build_tree(iter, inner_entry, ctx, args)?);
        }
        if root.leaves.is_empty() {
            root.push(Paint::new("empty").italic().dimmed().to_string());
        }
    } else if args.images {
        let images = cooklang_fs::RecipeEntry::try_from(entry)
            .map(|e| e.images())
            .ok();
        if let Some(images) = images {
            for img in images {
                root.push(Tree::new(
                    Paint::new(img.path.file_name().unwrap_or("-"))
                        .dimmed()
                        .to_string(),
                ));
            }
        }
    }
    Ok(root)
}

fn tree_label(ctx: &Context, args: &ListArgs, entry: &DirEntry) -> Result<String> {
    let name = if args.absolute_paths {
        entry.path().as_str()
    } else if args.paths {
        entry.path().strip_prefix(&ctx.base_dir).unwrap().as_str()
    } else {
        entry.file_stem()
    };

    let r = if entry.file_type().is_dir() {
        Paint::cyan(name).to_string()
    } else if args.check {
        let check = check_str(ctx.parser()?, &entry);
        format!("{name} [{check}]")
    } else {
        name.to_string()
    };
    Ok(r)
}

fn list_label(ctx: &Context, args: &ListArgs, entry: DirEntry) -> Result<(String, String, String)> {
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
                entry.file_stem()
            )
        } else {
            entry.file_stem().to_string()
        }
    };

    let check = if args.check {
        format!(" [{}]", check_str(ctx.parser()?, &entry))
    } else {
        String::new()
    };

    let images = if let Some(images) = args
        .images
        .then(|| cooklang_fs::RecipeEntry::try_from(entry).ok())
        .flatten()
        .map(|e| e.images().len())
    {
        if images > 0 {
            format!(" [{} image{}]", images, if images == 1 { "" } else { "s" })
        } else {
            format!(" [{}]", Paint::new("no images").dimmed())
        }
    } else {
        String::new()
    };

    Ok((name, check, images))
}

fn check_str(parser: &CooklangParser, entry: &DirEntry) -> Paint<&'static str> {
    cooklang_fs::RecipeEntry::try_from(entry.clone())
        .ok()
        .and_then(|e| e.read().ok())
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
