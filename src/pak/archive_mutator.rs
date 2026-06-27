use std::{
    collections::BTreeMap,
    fs::File,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{filetime::WindowsFileTime, io::StreamCopier};

use super::{
    Archive, ArchiveEntry, ArchiveEntryPayload, ArchivePathIndex, ArchiveWriter, BlockAlignment,
    PakFormat, PakPath, archive_payload_source::ArchivePayloadSource,
};

pub struct ArchiveMutator {
    path: PathBuf,
    format: PakFormat,
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
        let format = archive.header().format();
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
            format,
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
        let mut source = ArchivePayloadSource::disk_file(source_path)?;
        self.upsert_payload(&mut source, pak_path, allow_append)
    }

    pub fn contains_file(&self, pak_path: &PakPath) -> bool {
        self.path_index.resolve_entry_index(pak_path).is_some()
    }

    pub fn remove_file(&mut self, pak_path: &PakPath) -> Result<bool> {
        let Some(entry_index) = self.path_index.resolve_entry_index(pak_path) else {
            return Ok(false);
        };
        let removed = self.entries.remove(entry_index);
        self.add_unused_slot(removed.offset(), removed.slot_size())?;
        self.path_index = ArchivePathIndex::new(&self.entries)?;
        Ok(true)
    }

    pub fn upsert_from_entry(
        &mut self,
        source: &mut File,
        entry: &ArchiveEntry,
        pak_path: &PakPath,
        allow_append: bool,
    ) -> Result<bool> {
        let mut source = ArchivePayloadSource::pak_entry(source, entry);
        self.upsert_payload(&mut source, pak_path, allow_append)
    }

    pub fn upsert_bytes(
        &mut self,
        data: &[u8],
        pak_path: &PakPath,
        allow_append: bool,
    ) -> Result<bool> {
        let now = WindowsFileTime::from_system_time(std::time::SystemTime::now()).value();
        let mut source = ArchivePayloadSource::bytes(data, now, now);
        self.upsert_payload(&mut source, pak_path, allow_append)
    }

    pub fn finish(mut self) -> Result<u64> {
        let final_extras = self
            .extras
            .into_iter()
            .zip(self.consumed_extras)
            .filter_map(|(entry, consumed)| (!consumed).then_some(entry))
            .collect::<Vec<_>>();
        let final_len = ArchiveWriter::for_format(self.format).write_to(
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

    fn upsert_payload(
        &mut self,
        source: &mut ArchivePayloadSource<'_>,
        pak_path: &PakPath,
        allow_append: bool,
    ) -> Result<bool> {
        if let Some(entry_index) = self.path_index.resolve_entry_index(pak_path) {
            return self.replace_existing(entry_index, source, allow_append);
        }

        let pak_path = self.path_index.canonicalize_for_insert(pak_path)?;
        self.add_new(source, &pak_path, allow_append)?;
        Ok(false)
    }

    fn replace_existing(
        &mut self,
        entry_index: usize,
        source: &mut ArchivePayloadSource<'_>,
        allow_append: bool,
    ) -> Result<bool> {
        let source_size = source.size();
        let modify_time = source.modify_time();
        let old_slot_size = self.entries[entry_index].slot_size();
        if source_size <= old_slot_size {
            self.file
                .seek(SeekFrom::Start(self.entries[entry_index].offset()))
                .context("failed to seek replacement slot")?;
            let outcome = source.copy_to(&self.copier, &mut self.file)?;
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
        let outcome = source.copy_to(&self.copier, &mut self.file)?;
        self.entries[entry_index].replace_moved(
            new_offset,
            outcome.bytes(),
            padding,
            outcome.md5(),
            modify_time,
        );
        Ok(true)
    }

    fn add_new(
        &mut self,
        source: &mut ArchivePayloadSource<'_>,
        pak_path: &PakPath,
        allow_append: bool,
    ) -> Result<()> {
        let source_size = source.size();
        let (new_offset, padding) = self.allocate_slot(source_size, allow_append)?;
        self.file
            .seek(SeekFrom::Start(new_offset))
            .context("failed to seek new file slot")?;
        let outcome = source.copy_to(&self.copier, &mut self.file)?;
        if padding > 0 {
            self.copier
                .write_zero_padding(&mut self.file, padding as usize)?;
        }
        let payload = ArchiveEntryPayload::from_copy_outcome(
            new_offset,
            padding,
            &outcome,
            source.create_time(),
            source.modify_time(),
        );
        let entry_index = self.entries.len();
        self.entries
            .push(ArchiveEntry::file(pak_path.as_str(), &payload)?);
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
}
