use std::path::Path;

use anyhow::{Context, Result};

use crate::filetime::WindowsFileTime;

#[derive(Clone, Copy)]
pub(crate) struct ArchiveFileMetadata {
    size: u64,
    create_time: i64,
    modify_time: i64,
}

impl ArchiveFileMetadata {
    pub(crate) fn from_path(path: &Path) -> Result<Self> {
        let metadata = path
            .metadata()
            .with_context(|| format!("failed to stat {}", path.display()))?;
        let create_time =
            WindowsFileTime::from_system_time(metadata.created().or_else(|_| metadata.modified())?)
                .value();
        let modify_time = WindowsFileTime::from_system_time(metadata.modified()?).value();
        Ok(Self {
            size: metadata.len(),
            create_time,
            modify_time,
        })
    }

    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    pub(crate) fn create_time(&self) -> i64 {
        self.create_time
    }

    pub(crate) fn modify_time(&self) -> i64 {
        self.modify_time
    }
}
