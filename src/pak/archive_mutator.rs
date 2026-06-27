use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{filetime::WindowsFileTime, io::StreamCopier};

use super::{Archive, ArchiveEntry, ArchiveWriter, BlockAlignment, PakPath};

pub struct ArchiveMutator {
    path: PathBuf,
    file: File,
    entries: Vec<ArchiveEntry>,
    extras: Vec<ArchiveEntry>,
    fat_offset: u64,
    copier: StreamCopier,
}

impl ArchiveMutator {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let archive = Archive::open(path)?;
        let fat_offset = archive.header().fat_offset();
        let (_, entries, extras) = archive.into_parts();
        let file = File::options()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            entries,
            extras,
            fat_offset,
            copier: StreamCopier::default_large(),
        })
    }

    pub fn upsert_file(
        &mut self,
        source_path: &Path,
        pak_path: &PakPath,
        allow_append: bool,
    ) -> Result<bool> {
        let metadata = source_path
            .metadata()
            .with_context(|| format!("failed to stat {}", source_path.display()))?;
        let source_size = metadata.len();
        let modify_time = WindowsFileTime::from_system_time(metadata.modified()?).value();

        if let Some(entry_index) = self.find_entry_index(pak_path.as_str()) {
            return self.replace_existing(
                entry_index,
                source_path,
                source_size,
                modify_time,
                allow_append,
            );
        }

        let create_time =
            WindowsFileTime::from_system_time(metadata.created().or_else(|_| metadata.modified())?)
                .value();
        self.add_new_file(
            source_path,
            pak_path,
            source_size,
            create_time,
            modify_time,
            allow_append,
        )?;
        Ok(false)
    }

    pub fn contains_file(&self, pak_path: &PakPath) -> bool {
        self.find_entry_index(pak_path.as_str()).is_some()
    }

    pub fn finish(mut self) -> Result<u64> {
        let final_len = ArchiveWriter::xl_games().write_to(
            &mut self.file,
            self.fat_offset,
            &self.entries,
            &self.extras,
        )?;
        self.file
            .set_len(final_len)
            .with_context(|| format!("failed to set length on {}", self.path.display()))?;
        Ok(final_len)
    }

    fn replace_existing(
        &mut self,
        entry_index: usize,
        source_path: &Path,
        source_size: u64,
        modify_time: i64,
        allow_append: bool,
    ) -> Result<bool> {
        let old_slot_size = self.entries[entry_index].slot_size();
        if source_size <= old_slot_size {
            self.file
                .seek(SeekFrom::Start(self.entries[entry_index].offset()))
                .context("failed to seek replacement slot")?;
            let outcome = self
                .copier
                .copy_file_to_writer_with_md5(source_path, &mut self.file)?;
            self.entries[entry_index].replace_in_place(outcome.bytes(), outcome.md5(), modify_time);
            return Ok(true);
        }

        if !allow_append {
            bail!(
                "replacement is {source_size} bytes but current slot is only {old_slot_size} bytes"
            );
        }

        let old_entry = self.entries.remove(entry_index);
        let create_time = old_entry.create_time();
        self.extras.push(ArchiveEntry::unused(
            old_entry.offset(),
            old_entry.slot_size(),
        )?);
        let pak_path = PakPath::new(old_entry.name().to_string())?;
        self.add_new_file(
            source_path,
            &pak_path,
            source_size,
            create_time,
            modify_time,
            true,
        )?;
        Ok(true)
    }

    fn add_new_file(
        &mut self,
        source_path: &Path,
        pak_path: &PakPath,
        source_size: u64,
        create_time: i64,
        modify_time: i64,
        allow_append: bool,
    ) -> Result<()> {
        let (new_offset, padding) = self.allocate_slot(source_size, allow_append)?;
        self.file
            .seek(SeekFrom::Start(new_offset))
            .context("failed to seek new file slot")?;
        let outcome = self
            .copier
            .copy_file_to_writer_with_md5(source_path, &mut self.file)?;
        if padding > 0 {
            self.write_zeros(padding as usize)?;
        }
        self.entries.push(
            ArchiveEntry::builder(pak_path.as_str())
                .offset(new_offset)
                .size(outcome.bytes())
                .size_duplicate(outcome.bytes())
                .padding_size(padding)
                .md5(outcome.md5())
                .create_time(create_time)
                .modify_time(modify_time)
                .build()?,
        );
        Ok(())
    }

    fn allocate_slot(&mut self, source_size: u64, allow_append: bool) -> Result<(u64, u32)> {
        if let Some(reuse_index) = self
            .extras
            .iter()
            .position(|entry| entry.name() == "__unused__" && source_size <= entry.size())
        {
            let extra = self.extras.remove(reuse_index);
            return Ok((extra.offset(), (extra.size() - source_size) as u32));
        }

        if !allow_append {
            bail!("no existing free slot can hold {source_size} bytes");
        }

        let new_offset = self.fat_offset;
        let aligned_end = BlockAlignment::align_offset(new_offset + source_size);
        self.fat_offset = aligned_end;
        Ok((new_offset, (aligned_end - new_offset - source_size) as u32))
    }

    fn find_entry_index(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|entry| entry.name() == name)
    }

    fn write_zeros(&mut self, mut bytes: usize) -> Result<()> {
        const ZEROES: [u8; 8192] = [0; 8192];
        while bytes > 0 {
            let chunk = bytes.min(ZEROES.len());
            self.file
                .write_all(&ZEROES[..chunk])
                .context("failed to write pak payload padding")?;
            bytes -= chunk;
        }
        Ok(())
    }
}
