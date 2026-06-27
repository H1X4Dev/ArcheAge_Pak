use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PakPath {
    value: String,
}

impl PakPath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let mut value = value.into().replace('\\', "/");
        while let Some(stripped) = value.strip_prefix('/') {
            value = stripped.to_string();
        }
        ensure!(!value.is_empty(), "pak path cannot be empty");
        ensure!(!value.contains('\0'), "pak path contains a NUL byte");
        for segment in value.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                bail!("unsafe pak path segment in {value}");
            }
        }
        Ok(Self { value })
    }

    pub fn from_disk_relative(path: &Path, prefix: Option<&str>) -> Result<Self> {
        let mut parts = Vec::new();
        if let Some(prefix) = prefix {
            let prefix = Self::new(prefix.to_string())?;
            parts.push(prefix.value);
        }

        for component in path.components() {
            match component {
                Component::Normal(value) => parts.push(value.to_string_lossy().replace('\\', "/")),
                Component::CurDir => {}
                _ => bail!("source path cannot be packed safely: {}", path.display()),
            }
        }

        Self::new(parts.join("/"))
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn join_to(&self, root: &Path) -> Result<PathBuf> {
        let mut out = root.to_path_buf();
        for segment in self.value.split('/') {
            if segment.contains(':') {
                bail!("pak path segment is invalid on Windows: {}", self.value);
            }
            out.push(segment);
        }
        Ok(out)
    }
}
