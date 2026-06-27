use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::{ArchiveEntry, Header, PakFormat, RECORD_SIZE, RecordCodec};

#[derive(Clone, Debug)]
pub struct Archive {
    path: PathBuf,
    header: Header,
    entries: Vec<ArchiveEntry>,
    extras: Vec<ArchiveEntry>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_detected(path)
    }

    pub fn open_with_format(path: impl AsRef<Path>, format: PakFormat) -> Result<Self> {
        Self::open_inner(path, Some(format))
    }

    fn open_detected(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_inner(path, None)
    }

    fn open_inner(path: impl AsRef<Path>, format: Option<PakFormat>) -> Result<Self> {
        let path = path.as_ref();
        let mut file =
            File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
        let archive_len = file
            .metadata()
            .with_context(|| format!("failed to stat {}", path.display()))?
            .len();
        let header = if let Some(format) = format {
            Header::read_with_format(&mut file, archive_len, format)?
        } else {
            Header::read_from(&mut file, archive_len)?
        };
        let codec = RecordCodec::for_format(header.format());
        let mut encrypted = vec![0_u8; header.record_count() * RECORD_SIZE];
        file.seek(SeekFrom::Start(header.fat_offset()))
            .context("failed to seek to pak FAT")?;
        file.read_exact(&mut encrypted)
            .context("failed to read pak FAT records")?;

        let mut records = Vec::with_capacity(header.record_count());
        for chunk in encrypted.chunks_exact(RECORD_SIZE) {
            records.push(codec.decode(chunk)?);
        }
        let extras = if header.format().stores_extras_first() {
            let entries = records.split_off(header.extra_file_count());
            let extras = records;
            records = entries;
            extras
        } else {
            records.split_off(header.file_count())
        };

        Ok(Self::new(path.to_path_buf(), header, records, extras))
    }

    pub fn new(
        path: PathBuf,
        header: Header,
        entries: Vec<ArchiveEntry>,
        extras: Vec<ArchiveEntry>,
    ) -> Self {
        Self {
            path,
            header,
            entries,
            extras,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    pub fn extras(&self) -> &[ArchiveEntry] {
        &self.extras
    }

    pub fn find(&self, name: &str) -> Option<&ArchiveEntry> {
        self.entries.iter().find(|entry| entry.name() == name)
    }

    pub fn into_parts(self) -> (Header, Vec<ArchiveEntry>, Vec<ArchiveEntry>) {
        (self.header, self.entries, self.extras)
    }
}
