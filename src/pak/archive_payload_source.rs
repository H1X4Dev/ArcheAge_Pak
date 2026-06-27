use std::{fs::File, path::Path};

use anyhow::Result;

use crate::io::{CopyOutcome, StreamCopier};

use super::{ArchiveEntry, ArchiveFileMetadata};

pub(super) enum ArchivePayloadSource<'a> {
    DiskFile {
        path: &'a Path,
        metadata: ArchiveFileMetadata,
    },
    PakEntry {
        file: &'a mut File,
        entry: &'a ArchiveEntry,
    },
    Bytes {
        data: &'a [u8],
        create_time: i64,
        modify_time: i64,
    },
}

impl<'a> ArchivePayloadSource<'a> {
    pub(super) fn disk_file(path: &'a Path) -> Result<Self> {
        let metadata = ArchiveFileMetadata::from_path(path)?;
        Ok(Self::DiskFile { path, metadata })
    }

    pub(super) fn pak_entry(file: &'a mut File, entry: &'a ArchiveEntry) -> Self {
        Self::PakEntry { file, entry }
    }

    pub(super) fn bytes(data: &'a [u8], create_time: i64, modify_time: i64) -> Self {
        Self::Bytes {
            data,
            create_time,
            modify_time,
        }
    }

    pub(super) fn size(&self) -> u64 {
        match self {
            Self::DiskFile { metadata, .. } => metadata.size(),
            Self::PakEntry { entry, .. } => entry.size(),
            Self::Bytes { data, .. } => data.len() as u64,
        }
    }

    pub(super) fn create_time(&self) -> i64 {
        match self {
            Self::DiskFile { metadata, .. } => metadata.create_time(),
            Self::Bytes { create_time, .. } => *create_time,
            Self::PakEntry { entry, .. } => entry.create_time(),
        }
    }

    pub(super) fn modify_time(&self) -> i64 {
        match self {
            Self::DiskFile { metadata, .. } => metadata.modify_time(),
            Self::Bytes { modify_time, .. } => *modify_time,
            Self::PakEntry { entry, .. } => entry.modify_time(),
        }
    }

    pub(super) fn copy_to(
        &mut self,
        copier: &StreamCopier,
        writer: &mut File,
    ) -> Result<CopyOutcome> {
        match self {
            Self::DiskFile { path, .. } => copier.copy_file_to_writer_with_md5(path, writer),
            Self::PakEntry { file, entry } => {
                copier.copy_range_to_writer_with_md5(file, entry.offset(), entry.size(), writer)
            }
            Self::Bytes { data, .. } => copier.copy_bytes_to_writer_with_md5(data, writer),
        }
    }
}
