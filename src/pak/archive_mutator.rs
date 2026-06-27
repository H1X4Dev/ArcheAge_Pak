use std::{
    collections::BTreeMap,
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{filetime::WindowsFileTime, io::StreamCopier};

use super::{Archive, ArchiveEntry, ArchivePathIndex, ArchiveWriter, BlockAlignment, PakPath};

pub struct ArchiveMutator {
    path: PathBuf,
    file: File,
    entries: Vec<ArchiveEntry>,
    extras: Vec<ArchiveEntry>,
    consumed_extras: Vec<bool>,
    path_index: ArchivePathIndex,
    unused_slots: BTreeMap<u64, Vec<usize>>,
    fat_offset: u64,
    copier: StreamCopier,
}

impl ArchiveMutator {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let archive = Archive::open(path)?;
        let fat_offset = archive.header().fat_offset();
        let (_, entries, extras) = archive.into_parts();
        let path_index = ArchivePathIndex::new(&entries)?;
        let unused_slots = Self::build_unused_slots(&extras);
        let consumed_extras = vec![false; extras.len()];
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
            consumed_extras,
            path_index,
            unused_slots,
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

        if let Some(entry_index) = self.path_index.resolve_entry_index(pak_path) {
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
        let pak_path = self.path_index.canonicalize_for_insert(pak_path)?;
        self.add_new_file(
            source_path,
            &pak_path,
            source_size,
            create_time,
            modify_time,
            allow_append,
        )?;
        Ok(false)
    }

    pub fn contains_file(&self, pak_path: &PakPath) -> bool {
        self.path_index.resolve_entry_index(pak_path).is_some()
    }

    pub fn finish(mut self) -> Result<u64> {
        let final_extras = self
            .extras
            .into_iter()
            .zip(self.consumed_extras)
            .filter_map(|(entry, consumed)| (!consumed).then_some(entry))
            .collect::<Vec<_>>();
        let final_len = ArchiveWriter::xl_games().write_to(
            &mut self.file,
            self.fat_offset,
            &self.entries,
            &final_extras,
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

        let old_offset = self.entries[entry_index].offset();
        let old_slot_size = self.entries[entry_index].slot_size();
        self.add_unused_slot(old_offset, old_slot_size)?;
        let (new_offset, padding) = self.allocate_slot(source_size, true)?;
        self.file
            .seek(SeekFrom::Start(new_offset))
            .context("failed to seek replacement append slot")?;
        let outcome = self
            .copier
            .copy_file_to_writer_with_md5(source_path, &mut self.file)?;
        self.entries[entry_index].replace_moved(
            new_offset,
            outcome.bytes(),
            padding,
            outcome.md5(),
            modify_time,
        );
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
        let entry_index = self.entries.len();
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
        self.path_index
            .insert_file(pak_path.as_str(), entry_index)?;
        Ok(())
    }

    fn allocate_slot(&mut self, source_size: u64, allow_append: bool) -> Result<(u64, u32)> {
        if let Some(slot_size) = self
            .unused_slots
            .range(source_size..)
            .map(|(size, _)| *size)
            .next()
        {
            let indexes = self
                .unused_slots
                .get_mut(&slot_size)
                .context("unused slot index disappeared")?;
            let extra_index = indexes.pop().context("unused slot list was empty")?;
            if indexes.is_empty() {
                self.unused_slots.remove(&slot_size);
            }
            self.consumed_extras[extra_index] = true;
            let extra = &self.extras[extra_index];
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

    fn add_unused_slot(&mut self, offset: u64, slot_size: u64) -> Result<()> {
        let extra_index = self.extras.len();
        self.extras.push(ArchiveEntry::unused(offset, slot_size)?);
        self.consumed_extras.push(false);
        self.unused_slots
            .entry(slot_size)
            .or_default()
            .push(extra_index);
        Ok(())
    }

    fn build_unused_slots(extras: &[ArchiveEntry]) -> BTreeMap<u64, Vec<usize>> {
        let mut unused_slots = BTreeMap::<u64, Vec<usize>>::new();
        for (index, entry) in extras.iter().enumerate() {
            if entry.name() == "__unused__" {
                unused_slots.entry(entry.size()).or_default().push(index);
            }
        }
        unused_slots
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
