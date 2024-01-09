//! `cooklang-rs` helper crate.
//!
//! Utilities to deal with referencing recipe, images and data related to
//! recipes that are in other files.
//!
//! It implements an index into the file system ([FsIndex]) to efficiently
//! get recipes from a path. Also, get related images from a recipe.

mod walker;

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use camino::{Utf8Path, Utf8PathBuf};
use cooklang::quantity::QuantityValue;
use once_cell::sync::OnceCell;
use serde::Serialize;

pub use walker::DirEntry;
use walker::Walker;

/// Index of a directory for cooklang recipes
///
/// The index is lazy, so it will only search for things it needs when asked,
/// not when created.
#[derive(Debug)]
pub struct FsIndex {
    base_path: Utf8PathBuf,
    cache: RefCell<Cache>,
    walker: RefCell<Walker>,
}

#[derive(Debug, Default)]
struct Cache {
    recipes: HashMap<String, Vec<Utf8PathBuf>>,
    non_existent: VecDeque<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Recipe not found: '{0}'")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid name: '{0}'")]
    InvalidName(String),
    #[error(transparent)]
    NotRecipe(#[from] NotRecipe),
}

#[derive(Debug, thiserror::Error)]
#[error("Non UTF8 path")]
pub struct NonUtf8(std::path::PathBuf);

impl FsIndex {
    /// Create a new lazy index of the given path
    pub fn new(base_path: impl AsRef<std::path::Path>, max_depth: usize) -> Result<Self, Error> {
        let base_path: &Utf8Path = base_path
            .as_ref()
            .try_into()
            .map_err(|e: camino::FromPathError| e.into_io_error())?;
        let walker = Walker::new(base_path, max_depth);

        Ok(Self {
            base_path: base_path.into(),
            cache: Cache::default().into(),
            walker: walker.into(),
        })
    }

    /// Create a new complete index of the given path
    pub fn new_indexed(
        base_path: impl AsRef<std::path::Path>,
        max_depth: usize,
    ) -> Result<Self, Error> {
        let mut index = Self::new(base_path, max_depth)?;
        index.index_all()?;
        Ok(index)
    }

    /// Sets a config dir to the walker
    ///
    /// If this dir is found not in the top level, a warning will be printed.
    ///
    /// This also [Self::ignore]s the dir.
    pub fn set_config_dir(&mut self, dir: String) {
        self.walker.get_mut().set_config_dir(dir);
    }

    /// Ignores a given file/dir
    pub fn ignore(&mut self, dir: String) {
        self.walker.get_mut().ignore(dir);
    }

    pub fn base_path(&self) -> &Utf8Path {
        &self.base_path
    }

    /// Check if the index contains a recipe
    pub fn contains(&self, recipe: &str) -> bool {
        self.get(recipe).is_ok()
    }

    /// Completes the lazy indexing
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn index_all(&mut self) -> Result<(), Error> {
        for entry in self.walker.get_mut() {
            let entry = entry?;
            let Some((entry_name, path)) = process_entry(&entry) else {
                continue;
            };
            self.cache.borrow_mut().insert(entry_name, path);
        }
        Ok(())
    }

    /// Get a recipe from the index
    ///
    /// The input recipe is a partial path with or without the .cook extension.
    #[tracing::instrument(level = "debug", name = "fs_index_get", skip(self))]
    pub fn get(&self, recipe: &str) -> Result<RecipeEntry, Error> {
        let (name, path) = into_name_path(recipe)?;

        // Is in cache?
        if let Some(path) = self.cache.borrow().get(&name, &path) {
            return Ok(RecipeEntry::new(path));
        }
        if self.cache.borrow().non_existent.iter().any(|r| r == recipe) {
            return Err(Error::NotFound(recipe.to_string()));
        }

        // Walk until found or no more files
        // as walk is breadth-first and sorted by filename, the first found will
        // be the wanted: outermost alphabetically
        while let Some(entry) = self.walker.borrow_mut().next() {
            let entry = entry?;

            let Some((entry_name, entry_path)) = process_entry(&entry) else {
                continue;
            };

            // Add to cache
            self.cache.borrow_mut().insert(entry_name, entry_path);

            if compare_path(entry_path, &path) {
                return Ok(RecipeEntry::new(entry_path.into()));
            }
        }

        self.cache.borrow_mut().mark_non_existent(recipe);
        Err(Error::NotFound(recipe.to_string()))
    }

    /// Remove a recipe from the cache
    ///
    /// The path cannot contain the current dir (`.`) or the parent
    /// dir (`..`).
    ///
    /// Remember that the the indexing procedure is lazy, so further calls to
    /// [FsIndex::get] may discover the removed recipe if it was not indexed
    /// before.
    ///
    /// To avoid this, call [FsIndex::index_all] to index everything before
    /// removing or [FsIndex::add_recipe].
    ///
    /// # Errors
    /// The only possible is [Error::InvalidName].
    ///
    /// # Panics
    /// - If the path does not start with the base path
    pub fn remove_recipe(&mut self, path: &Utf8Path) -> Result<(), Error> {
        tracing::trace!("manually removing {path}");
        assert!(
            path.starts_with(&self.base_path),
            "path does not start with the base path"
        );

        let (name, path) = into_name_path(path.as_str())?;
        self.cache.get_mut().remove(&name, &path);
        Ok(())
    }

    /// Manually add a recipe to the cache.
    ///
    /// The path cannot contain the current dir (`.`) or the parent
    /// dir (`..`). The file must exist.
    ///
    /// # Errors
    /// The only possible is [Error::InvalidName].
    ///
    /// # Panics
    /// - If the path does not start with the base path
    /// - If the file does not exist.
    pub fn add_recipe(&mut self, path: &Utf8Path) -> Result<(), Error> {
        tracing::trace!("manually adding {path}");
        assert!(
            path.starts_with(&self.base_path),
            "path does not start with the base path"
        );
        assert!(path.is_file(), "path does not exist or is not a file");

        // if its known, do nothing
        if self.get(path.as_str()).is_ok() {
            return Ok(());
        }

        let (name, path) = into_name_path(path.as_str())?;
        self.cache.get_mut().insert(&name, &path);
        Ok(())
    }
}

fn process_entry(dir_entry: &DirEntry) -> Option<(&str, &Utf8Path)> {
    // Ignore non files or not .cook files
    if !dir_entry.is_cooklang_file() {
        return None;
    }

    let entry_name = dir_entry.file_stem();

    Some((entry_name, dir_entry.path()))
}

impl Cache {
    /// args should be lowercase already
    fn get(&self, name: &str, path: &Utf8Path) -> Option<Utf8PathBuf> {
        let paths = self.recipes.get(&name.to_lowercase())?;
        paths.iter().find(|p| compare_path(p, path)).cloned()
    }

    fn insert(&mut self, name: &str, path: &Utf8Path) {
        tracing::trace!("adding {name}:{path} to index cache");
        let recipes = self.recipes.entry(name.to_lowercase()).or_default();
        let pos = recipes.partition_point(|p| {
            // less components first. same, alphabetically
            match p.components().count().cmp(&path.components().count()) {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Equal => p.as_str() < path.as_str(),
                std::cmp::Ordering::Greater => false,
            }
        });
        recipes.insert(pos, path.to_path_buf());
        if let Some(marked) = self.non_existent.iter().position(|p| p == path.as_str()) {
            self.non_existent.remove(marked);
        }
    }

    fn remove(&mut self, name: &str, path: &Utf8Path) {
        tracing::trace!("removing {name}:{path} from index cache");
        if let Some(recipes) = self.recipes.get_mut(&name.to_lowercase()) {
            // can't do swap so "outer" recipes remain first
            if let Some(index) = recipes.iter().position(|r| r == path) {
                recipes.remove(index);
            }
        }
    }

    const NON_EXISTENT_CACHE_SIZE: usize = 10;

    fn mark_non_existent(&mut self, path: &str) {
        if self.non_existent.len() == Self::NON_EXISTENT_CACHE_SIZE {
            self.non_existent.pop_front();
        }
        self.non_existent.push_back(path.into());
    }
}

fn into_name_path(recipe: &str) -> Result<(String, Utf8PathBuf), Error> {
    let path = Utf8PathBuf::from(recipe);
    let name = path
        .file_stem()
        .ok_or_else(|| Error::InvalidName(recipe.into()))?
        .to_string();
    Ok((name, path))
}

fn compare_path_key(p: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(p.as_str().to_lowercase()).with_extension("")
}

fn compare_path(full: &Utf8Path, suffix: &Utf8Path) -> bool {
    // only compare the end, so partial paths are a valid form of referencing recipes
    compare_path_key(full).ends_with(compare_path_key(suffix))
}

/// Get all recipes from a path with a depth limit
pub fn all_recipes(
    base_path: impl AsRef<std::path::Path>,
    max_depth: usize,
) -> Result<impl Iterator<Item = RecipeEntry>, std::io::Error> {
    let base_path: &Utf8Path = base_path
        .as_ref()
        .try_into()
        .map_err(|e: camino::FromPathError| e.into_io_error())?;
    let walker = Walker::new(base_path, max_depth).flatten();
    let grouped = group_images(walker);
    Ok(grouped.filter_map(|e| match e {
        Entry::Dir(_) => None,
        Entry::Recipe(r) => Some(r),
    }))
}

/// Walks a single directory retrieving recipes and other directories
pub fn walk_dir(
    path: impl AsRef<std::path::Path>,
) -> Result<impl Iterator<Item = Entry>, std::io::Error> {
    let path: &Utf8Path = path
        .as_ref()
        .try_into()
        .map_err(|e: camino::FromPathError| e.into_io_error())?;
    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "dir not found",
        ));
    }
    Ok(group_images(Walker::new(path, 0).flatten()))
}

fn group_images(walker: impl Iterator<Item = DirEntry>) -> impl Iterator<Item = Entry> {
    struct ImageGrouper<I: Iterator<Item = DirEntry>> {
        iter: std::iter::Peekable<I>,
    }

    impl<I: Iterator<Item = DirEntry>> Iterator for ImageGrouper<I> {
        type Item = Entry;

        fn next(&mut self) -> Option<Self::Item> {
            let mut past_images = Vec::new();
            loop {
                match self.iter.next()? {
                    dir if dir.file_type().is_dir() => return Some(Entry::Dir(dir)),
                    r if r.is_cooklang_file() => {
                        let recipe_name = r.file_stem();
                        // because file are sorted by name, recipe images will be with the
                        // recipes
                        let mut images = past_images
                            .into_iter()
                            .filter_map(|e| Image::new(recipe_name, e))
                            .collect::<Vec<_>>();
                        while let Some(image_entry) = self.iter.next_if(|e| e.is_image()) {
                            if let Some(image) = Image::new(recipe_name, image_entry) {
                                images.push(image);
                            }
                        }
                        return Some(Entry::Recipe(
                            RecipeEntry::new(r.into_path()).set_images(images),
                        ));
                    }
                    img if img.is_image() => {
                        past_images.push(img);
                    }
                    _ => {}
                }
            }
        }
    }

    ImageGrouper {
        iter: walker.peekable(),
    }
}

pub enum Entry {
    Dir(DirEntry),
    Recipe(RecipeEntry),
}

/// Resolves a recipe query first trying directly as a path and if it fails performs
/// a lookup in the index.
///
/// The path can be outside the indexed dir.
#[tracing::instrument(level = "debug", skip(index), ret, err)]
pub fn resolve_recipe(
    query: &str,
    index: &FsIndex,
    relative_to: Option<&Utf8Path>,
) -> Result<RecipeEntry, Error> {
    fn as_path(query: &str, relative_to: Option<&Utf8Path>) -> Result<RecipeEntry, Error> {
        let mut path = Utf8PathBuf::from(query);

        if let Some(base) = relative_to {
            if path.is_relative() {
                path = base.join(path);
            }
        }

        DirEntry::new(&path)
            .map_err(Error::from)
            .and_then(|e| RecipeEntry::try_from(e).map_err(Error::from))
    }

    as_path(query, relative_to).or_else(|_| index.get(query))
}

#[derive(Debug, Clone)]
pub struct RecipeEntry {
    path: Utf8PathBuf,
    images: OnceCell<Vec<Image>>,
    content: OnceCell<RecipeContent>,
}

impl RecipeEntry {
    pub fn new(path: Utf8PathBuf) -> Self {
        Self {
            path,
            images: OnceCell::new(),
            content: OnceCell::new(),
        }
    }

    pub fn set_images(self, images: Vec<Image>) -> Self {
        Self {
            images: OnceCell::with_value(images),
            ..self
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn file_name(&self) -> &str {
        self.path.file_name().unwrap()
    }

    pub fn name(&self) -> &str {
        self.path.file_stem().unwrap()
    }

    pub fn relative_name(&self) -> &str {
        self.path.as_str().trim_end_matches(".cook")
    }

    /// Reads the content of the entry
    ///
    /// The result is cached.
    pub fn read(&self) -> std::io::Result<&RecipeContent> {
        self.content.get_or_try_init(|| {
            let content = std::fs::read_to_string(&self.path)?;
            Ok(RecipeContent::new(content))
        })
    }

    /// Finds the images of the recipe
    ///
    /// The result is cached, use the [`recipe_images`] to get a fresh result
    /// each call.
    pub fn images(&self) -> &[Image] {
        self.images.get_or_init(|| recipe_images(&self.path))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The entry is not a recipe: {0}")]
pub struct NotRecipe(Utf8PathBuf);
impl TryFrom<DirEntry> for RecipeEntry {
    type Error = NotRecipe;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        if !value.is_cooklang_file() {
            return Err(NotRecipe(value.into_path()));
        }
        Ok(Self::new(value.into_path()))
    }
}

#[derive(Debug, Clone)]
pub struct RecipeContent {
    content: String,
}

impl RecipeContent {
    fn new(content: String) -> Self {
        Self { content }
    }

    /// Parses the metadata of the recipe
    ///  
    /// The result is cached
    pub fn metadata(&self, parser: &cooklang::CooklangParser) -> cooklang::MetadataResult {
        parser.parse_metadata(&self.content)
    }

    /// Parses the recipe
    ///
    /// This is an alias for [`Self::parse_with_recipe_ref_checker`] with no
    /// chcker.
    ///
    /// The result is cached, so if you cange the checker, you won't see any
    /// change.
    pub fn parse(&self, parser: &cooklang::CooklangParser) -> cooklang::RecipeResult {
        self.parse_with_recipe_ref_checker(parser, None)
    }

    /// Parses the recipe checking referenced recipes
    ///
    /// The result is cached, so if you cange the checker, you won't see any
    /// change.
    pub fn parse_with_recipe_ref_checker(
        &self,
        parser: &cooklang::CooklangParser,
        checker: Option<cooklang::RecipeRefChecker>,
    ) -> cooklang::RecipeResult {
        parser.parse_with_recipe_ref_checker(&self.content, checker)
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn into_text(self) -> String {
        self.content
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Image {
    pub indexes: Option<ImageIndexes>,
    pub path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ImageIndexes {
    section: u16,
    step: u16,
}

impl Image {
    fn new(recipe_name: &str, entry: DirEntry) -> Option<Self> {
        let parts = entry.file_name().rsplitn(4, '.').collect::<Vec<_>>();

        // no dots, so no extension
        if parts.len() == 1 {
            return None;
        }

        let name = *parts.last().unwrap();
        let ext = *parts.first().unwrap();

        if name != recipe_name || !IMAGE_EXTENSIONS.contains(&ext) {
            return None;
        }

        let indexes = match &parts[1..parts.len() - 1] {
            [step, section] => {
                let section = section.parse::<u16>().ok()?;
                let step = step.parse::<u16>().ok()?;
                Some(ImageIndexes { section, step })
            }
            [step] => {
                let step = step.parse::<u16>().ok()?;
                Some(ImageIndexes { section: 0, step })
            }
            _ => None,
        };

        Some(Image {
            indexes,
            path: entry.into_path(),
        })
    }
}

/// Valid image extensions
pub const IMAGE_EXTENSIONS: &[&str] = &["jpeg", "jpg", "png", "heic", "gif", "webp"];

/// Get a list of the images of the recipe
///
/// See [IMAGE_EXTENSIONS].
pub fn recipe_images(path: &Utf8Path) -> Vec<Image> {
    let Some(dir) = path.parent().and_then(|dir| dir.read_dir_utf8().ok()) else {
        return vec![];
    };

    let Some(recipe_name) = path.file_stem() else {
        return vec![];
    };

    let mut images = dir
        .filter_map(|e| e.ok()) // skip error
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false)) // skip non-file
        .filter_map(|e| Image::new(recipe_name, DirEntry::new(e.path()).ok()?))
        .collect::<Vec<_>>();
    images.sort_unstable();
    images
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeImageError {
    #[error("No section {section} in recipe, referenced from {image}")]
    MissingSection { section: u16, image: Utf8PathBuf },
    #[error("No step {step} in section {section}, referenced from {image}")]
    MissingStep {
        section: u16,
        step: u16,
        image: Utf8PathBuf,
    },
}

/// Check that all images for a recipe actually can reference it.
///
/// For example the image `Recipe.14.jpeg` references step 15th, but the
/// recipe may not have 15 steps, so this function returns an error.
pub fn check_recipe_images<D, V: QuantityValue>(
    images: &[Image],
    recipe: &cooklang::Recipe<D, V>,
) -> Result<(), Vec<RecipeImageError>> {
    let mut errors = Vec::new();
    for image in images {
        if let Some(ImageIndexes { section, step }) = image.indexes {
            let Some(recipe_section) = recipe.sections.get(section as usize) else {
                errors.push(RecipeImageError::MissingSection {
                    section,
                    image: image.path.clone(),
                });
                continue;
            };

            if step as usize >= recipe_section.content.len() {
                errors.push(RecipeImageError::MissingStep {
                    section,
                    step,
                    image: image.path.clone(),
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
