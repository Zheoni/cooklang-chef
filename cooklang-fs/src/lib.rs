use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs::FileType,
};

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};

#[derive(Debug)]
pub struct FsIndex {
    base_path: PathBuf,
    cache: RefCell<Cache>,
    walker: RefCell<walkdir::IntoIter>,
}

#[derive(Debug, Default)]
struct Cache {
    recipes: HashMap<String, Vec<PathBuf>>,
    non_existent: HashSet<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Recipe not found: '{0}'")]
    NotFound(String),
    #[error(transparent)]
    Walk(#[from] walkdir::Error),
    #[error("Error canonicalizing path")]
    Canonicalize(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("Invalid name: '{0}'")]
    InvalidName(String),
    #[error(transparent)]
    NonUtf8(#[from] NonUtf8),
}

pub struct RecipeEntry(PathBuf);

#[derive(Debug, Clone)]
pub struct DirEntry {
    path: PathBuf,
    depth: usize,
    file_type: FileType,
}

impl DirEntry {
    pub fn file_name(&self) -> &str {
        self.path.file_name().unwrap_or(self.path.as_str())
    }
    pub fn file_stem(&self) -> &str {
        self.path.file_stem().unwrap_or(self.path.as_str())
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file_type(&self) -> FileType {
        self.file_type
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The entry is not a recipe: {}", .0.path)]
pub struct NotRecipe(DirEntry);
impl TryFrom<DirEntry> for RecipeEntry {
    type Error = NotRecipe;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        if !is_cooklang_file(&value) {
            return Err(NotRecipe(value));
        }
        Ok(Self(value.path))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Non UTF8 path")]
pub struct NonUtf8(std::path::PathBuf);

impl TryFrom<walkdir::DirEntry> for DirEntry {
    type Error = NonUtf8;

    fn try_from(value: walkdir::DirEntry) -> Result<Self, Self::Error> {
        let depth = value.depth();
        let file_type = value.file_type();
        let path = PathBuf::from_path_buf(value.into_path()).map_err(|p| NonUtf8(p))?;
        Ok(Self {
            path,
            depth,
            file_type,
        })
    }
}

impl FsIndex {
    pub fn new(base_path: impl AsRef<std::path::Path>, max_depth: usize) -> Result<Self, Error> {
        let base_path = Path::from_path(base_path.as_ref())
            .ok_or_else(|| Error::NonUtf8(NonUtf8(base_path.as_ref().into())))?;
        let walker = walkdir::WalkDir::new(&base_path)
            .max_depth(max_depth)
            .sort_by(
                // files first, and sort by name
                |a, b| match (a.file_type().is_file(), b.file_type().is_file()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                },
            )
            .into_iter();

        Ok(Self {
            base_path: base_path.into(),
            cache: Cache::default().into(),
            walker: walker.into(),
        })
    }

    pub fn all(
        base_path: impl AsRef<std::path::Path>,
        max_depth: usize,
    ) -> impl Iterator<Item = DirEntry> {
        walkdir::WalkDir::new(base_path.as_ref())
            .max_depth(max_depth)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok().and_then(|e| DirEntry::try_from(e).ok()))
            .filter(|e| e.file_type.is_dir() || is_cooklang_file(&e))
    }

    pub fn contains(&self, recipe: &str) -> bool {
        self.get(recipe).is_ok()
    }

    #[tracing::instrument(name = "fs_index_get", skip(self))]
    pub fn get(&self, recipe: &str) -> Result<RecipeEntry, Error> {
        let path = Path::new(recipe);
        let name = path
            .file_stem()
            .ok_or_else(|| Error::InvalidName(recipe.into()))?;

        // Is in cache?
        if let Some(path) = self.cache.borrow().get(name, path) {
            return Ok(RecipeEntry(path));
        }
        if self.cache.borrow().non_existent.contains(recipe) {
            return Err(Error::NotFound(recipe.to_string()));
        }

        // Is a file relative to base?
        let possible_path = self.base_path.join(recipe).with_extension("cook");
        if possible_path.is_file() {
            // Add to cache
            self.cache.borrow_mut().insert(name, path);
            return Ok(RecipeEntry(path.into()));
        }

        // Walk until found or no more files
        while let Some(entry) = self.walker.borrow_mut().next() {
            let entry = entry?;
            let entry = DirEntry::try_from(entry)?;

            let Some((entry_name, path)) = process_entry(&entry) else { continue; };

            // Add to cache
            self.cache.borrow_mut().insert(entry_name, path);

            if entry_name == name {
                return Ok(RecipeEntry(path.into()));
            }
        }

        self.cache.borrow_mut().mark_non_existent(recipe);
        Err(Error::NotFound(recipe.to_string()))
    }
}

fn is_cooklang_file(dir_entry: &DirEntry) -> bool {
    dir_entry.file_type.is_file()
        && dir_entry
            .path
            .extension()
            .map(|e| e == "cook")
            .unwrap_or(false)
}

fn process_entry(dir_entry: &DirEntry) -> Option<(&str, &Path)> {
    // Ignore non files or not .cook files
    if !is_cooklang_file(dir_entry) {
        return None;
    }

    let entry_name = dir_entry.file_stem();

    Some((entry_name, &dir_entry.path))
}

impl Cache {
    fn get(&self, name: &str, path: &Path) -> Option<PathBuf> {
        let v = self.recipes.get(name)?;
        v.iter().find(|&p| p == path).cloned()
    }

    fn insert(&mut self, name: &str, path: &Path) {
        self.recipes
            .entry(name.to_string())
            .or_default()
            .push(path.into())
    }

    fn mark_non_existent(&mut self, recipe: &str) {
        self.non_existent.insert(recipe.into());
    }
}

impl RecipeEntry {
    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn read(&self) -> std::io::Result<RecipeContent> {
        let content = std::fs::read_to_string(&self.0)?;
        Ok(RecipeContent {
            content,
            path: self.0.clone(),
        })
    }

    pub fn images(&self) -> Vec<Image> {
        recipe_images(&self.0)
    }
}

pub struct RecipeContent {
    content: String,
    path: PathBuf,
}

impl RecipeContent {
    pub fn metadata(&self, parser: &cooklang::CooklangParser) -> cooklang::MetadataResult {
        parser.parse_metadata(&self.content)
    }

    pub fn parse(&self, parser: &cooklang::CooklangParser) -> cooklang::RecipeResult {
        parser.parse(
            &self.content,
            self.path.file_stem().expect("empty recipe name").as_ref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Image {
    pub indexes: Option<(usize, usize)>,
    pub path: PathBuf,
}

pub fn recipe_images(path: &Path) -> Vec<Image> {
    let Some(dir) = path.parent() else { return vec![]; };
    let mut images = walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| PathBuf::from_path_buf(e.path().to_path_buf()).ok())
        .filter(|e| !e.is_dir())
        .filter_map(|image_path| {
            const EXTENSIONS: &[&str] = &["jpeg", "jpg", "png", "heic", "gif", "webp"];
            let parts = image_path
                .file_name()
                .unwrap()
                .rsplitn(4, '.')
                .collect::<Vec<_>>();
            let recipe_name = path
                .file_name()
                .unwrap_or_default()
                .split_once('.')
                .map(|s| s.0)
                .unwrap_or_default();
            match parts.as_slice() {
                &[ext, step_index, section_index, name] => {
                    let step_index = step_index.parse::<usize>();
                    let section_index = section_index.parse::<usize>();
                    (EXTENSIONS.contains(&ext)
                        && name == recipe_name
                        && step_index.is_ok()
                        && section_index.is_ok())
                    .then_some(Image {
                        indexes: Some((section_index.unwrap(), step_index.unwrap())),
                        path: image_path,
                    })
                }
                &[ext, step_index, name] => {
                    let step_index = step_index.parse::<usize>();
                    (EXTENSIONS.contains(&ext) && name == recipe_name && step_index.is_ok())
                        .then_some(Image {
                            indexes: Some((0, step_index.unwrap())),
                            path: image_path,
                        })
                }
                &[ext, name] => {
                    (EXTENSIONS.contains(&ext) && name == recipe_name).then_some(Image {
                        indexes: None,
                        path: image_path,
                    })
                }
                [_name] => None,
                _ => unreachable!(),
            }
        })
        .collect::<Vec<_>>();
    images.sort_unstable();
    images
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeImageError {
    #[error("No section {section} in recipe, referenced from {image}")]
    MissingSection { section: usize, image: PathBuf },
    #[error("No step {step} in section {section}, referenced from {image}")]
    MissingStep {
        section: usize,
        step: usize,
        image: PathBuf,
    },
}

pub fn check_recipe_images<D>(
    images: &[Image],
    recipe: &cooklang::Recipe<D>,
) -> Result<(), Vec<RecipeImageError>> {
    let mut errors = Vec::new();
    for image in images {
        if let Some((section_index, step_index)) = image.indexes {
            let Some(section) = recipe.sections.get(section_index)
            else {
                errors.push(RecipeImageError::MissingSection {
                    section: section_index,
                    image: image.path.clone()
                });
                continue;
            };

            if step_index >= section.steps.len() {
                errors.push(RecipeImageError::MissingStep {
                    section: section_index,
                    step: step_index,
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn mock_fs() -> FsIndex {
//         let mut fs = FsIndex::empty("");
//         macro_rules! e {
//             ($path:expr) => {
//                 let path = Path::new($path);
//                 fs.insert(
//                     path.file_stem().unwrap().to_string_lossy().as_ref(),
//                     path,
//                     path.components().count(),
//                 );
//             };
//         }
//         e!("recipe.cook");
//         e!("a/recipe.cook");
//         e!("a/b/other.cook");
//         e!("a/b/c/recipe.cook");
//         e!("d/other.cook");
//         fs
//     }

//     #[test]
//     fn test_get() {
//         let fs = mock_fs();

//         assert_eq!(fs.get("recipe").unwrap().path(), "recipe.cook");
//         assert_eq!(fs.get("other").unwrap().path(), "d/other.cook");
//         assert_eq!(fs.get("a/recipe").unwrap().path(), "a/recipe.cook");
//         assert_eq!(fs.get("a/recipe.cook").unwrap().path(), "a/recipe.cook");
//         assert_eq!(fs.get("a/b/c/recipe").unwrap().path(), "a/b/c/recipe.cook");

//         assert!(fs.get("croissant").is_err());
//         assert!(fs.get("a/b/recipe").is_err());
//     }
// }
