use std::{collections::VecDeque, fs::FileType};

use camino::{Utf8Path, Utf8PathBuf};

/// Breadth-first, sorted by file name, .cook filtered, dir walker.
///
/// Paths are relative to the base path, with the base path included. So when
/// walking over `dir`, entries will be `dir/whatever.cook`.
///
/// Files/dirs starting with '.' are ignored.
///
/// Currently, all [DirEntry] are guaranteed to be [RecipeEntry](super::RecipeEntry),
/// but this may change in the future.
#[derive(Debug)]
pub struct Walker {
    base_path: Utf8PathBuf,
    max_depth: usize,
    dirs: VecDeque<Utf8PathBuf>,
    current: std::vec::IntoIter<DirEntry>,
    config_dir: Option<String>,
    ignore: Vec<String>,
}

impl Walker {
    pub fn new(dir: impl AsRef<Utf8Path>, max_depth: usize) -> Self {
        let dir = dir.as_ref();
        let mut dirs = VecDeque::new();
        dirs.push_back(dir.to_path_buf());
        Self {
            base_path: dir.to_path_buf(),
            max_depth,
            dirs,
            current: Vec::new().into_iter(),
            config_dir: None,
            ignore: Vec::new(),
        }
    }

    /// Sets a config dir to the walker
    ///
    /// If this dir is found not in the top level, a warning will be printed.
    ///
    /// This also [Self::ignore]s the dir.
    pub fn set_config_dir(&mut self, dir: String) {
        if !dir.starts_with('.') {
            self.ignore.push(dir.clone());
        }
        self.config_dir = Some(dir);
    }

    /// Ignores a given file/dir
    pub fn ignore(&mut self, dir: String) {
        self.ignore.push(dir);
    }

    fn process_dir(&mut self, dir: &Utf8Path) -> Result<(), std::io::Error> {
        // the entire dir needs to be processed as one because entry order
        // is not guaranteed, so we need to sort
        let mut new_dirs = Vec::new();
        let mut new_entries = Vec::new();
        for e in dir.read_dir_utf8()? {
            let e = e?;
            let ft = e.file_type()?;

            // print warning for unexpected config dir
            if let Some(config_dir) = &self.config_dir {
                if ft.is_dir()
                    && e.file_name() == config_dir
                    && entry_depth(e.path(), &self.base_path) > 1
                {
                    tracing::warn!("Config dir `{config_dir}` found not in base path. It will be ignored. You may be running the application in the wrong directory.");
                }
            }

            // filter dot files/dirs and explicit filters
            if e.file_name().starts_with('.') || self.ignore.iter().any(|d| d == e.file_name()) {
                continue;
            }

            if ft.is_dir() {
                let depth = entry_depth(e.path(), &self.base_path);
                if depth <= self.max_depth {
                    new_dirs.push(e.into_path());
                }
            } else {
                if e.path().extension() != Some("cook") {
                    continue;
                }
                new_entries.push(DirEntry {
                    path: e.into_path(),
                    file_type: ft,
                });
            }
        }
        new_dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        new_entries.sort_by(|a, b| a.file_name().cmp(b.file_name()));
        self.dirs.extend(new_dirs);
        self.current = new_entries.into_iter();
        Ok(())
    }
}

impl Iterator for Walker {
    type Item = Result<DirEntry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // take from que queue
        if let Some(entry) = self.current.next() {
            return Some(Ok(entry));
        }

        // if none, take a dir from the queue and process it's contents
        while let Some(dir) = self.dirs.pop_front() {
            if let Err(e) = self.process_dir(&dir) {
                return Some(Err(e));
            }
            if let Some(entry) = self.current.next() {
                return Some(Ok(entry));
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    path: Utf8PathBuf,
    file_type: FileType,
}

impl DirEntry {
    pub fn new(path: &Utf8Path) -> Result<Self, std::io::Error> {
        let metadata = path.metadata()?;
        Ok(Self {
            path: path.to_path_buf(),
            file_type: metadata.file_type(),
        })
    }

    pub fn file_name(&self) -> &str {
        self.path.file_name().unwrap_or(self.path.as_str())
    }
    pub fn file_stem(&self) -> &str {
        self.path.file_stem().unwrap_or(self.path.as_str())
    }
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
    pub fn into_path(self) -> Utf8PathBuf {
        self.path
    }
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn is_cooklang_file(&self) -> bool {
        self.file_type.is_file() && self.path.extension().map(|e| e == "cook").unwrap_or(false)
    }
}

/// Calculates the depth of a path in relation to a base path.
///
/// # Panics
/// If `path` is not a suffix of `base_path`.
fn entry_depth(path: &Utf8Path, base_path: &Utf8Path) -> usize {
    path.strip_prefix(base_path)
        .expect("entry path not under base path")
        .components()
        .count()
}
