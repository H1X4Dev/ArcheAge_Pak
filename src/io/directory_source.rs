use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use walkdir::WalkDir;

pub struct DirectorySource {
    root: PathBuf,
}

impl DirectorySource {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        ensure!(
            root.is_dir(),
            "source directory does not exist: {}",
            root.display()
        );
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(&self.root).follow_links(false) {
            let entry = entry.with_context(|| format!("failed to walk {}", self.root.display()))?;
            if entry.file_type().is_file() {
                files.push(entry.into_path());
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
