use std::{
    fs::File,
    io::{Seek, SeekFrom},
};

use anyhow::{Context, Result, bail};

use crate::{
    cli::ReplaceArgs,
    filetime::WindowsFileTime,
    io::StreamCopier,
    pak::{Archive, ArchiveEntry, ArchiveWriter, BLOCK_SIZE, PakPath},
};

pub struct ReplaceCommand {
    args: ReplaceArgs,
}

impl ReplaceCommand {
    pub fn new(args: ReplaceArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let archive = Archive::open(&self.args.pak)?;
        let original_fat_offset = archive.header().fat_offset();
        let (_, mut entries, mut extras) = archive.into_parts();
        let pak_path = PakPath::new(self.args.pak_path.clone())?;
        let entry_index = entries
            .iter()
            .position(|entry| entry.name() == pak_path.as_str())
            .with_context(|| format!("file not found in pak: {}", pak_path.as_str()))?;
        let source_metadata = self.args.source_file.metadata().with_context(|| {
            format!(
                "failed to stat replacement file {}",
                self.args.source_file.display()
            )
        })?;
        let source_size = source_metadata.len();
        let modify_time = WindowsFileTime::from_system_time(source_metadata.modified()?).value();
        let mut pak = File::options()
            .read(true)
            .write(true)
            .open(&self.args.pak)
            .with_context(|| format!("failed to open {}", self.args.pak.display()))?;
        let copier = StreamCopier::default_large();
        let old_slot_size = entries[entry_index].slot_size();

        let new_fat_offset = if source_size <= old_slot_size {
            pak.seek(SeekFrom::Start(entries[entry_index].offset()))
                .context("failed to seek replacement slot")?;
            let outcome = copier.copy_file_to_writer_with_md5(&self.args.source_file, &mut pak)?;
            entries[entry_index].replace_in_place(outcome.bytes(), outcome.md5(), modify_time);
            original_fat_offset
        } else {
            if self.args.in_place_only {
                bail!(
                    "replacement is {} bytes but current slot is only {} bytes",
                    source_size,
                    old_slot_size
                );
            }
            let old_entry = entries.remove(entry_index);
            extras.push(ArchiveEntry::unused(
                old_entry.offset(),
                old_entry.slot_size(),
            )?);

            let reuse_index = extras
                .iter()
                .position(|entry| entry.name() == "__unused__" && source_size <= entry.size());
            let (new_offset, padding, fat_offset) = if let Some(reuse_index) = reuse_index {
                let extra = extras.remove(reuse_index);
                (
                    extra.offset(),
                    (extra.size() - source_size) as u32,
                    original_fat_offset,
                )
            } else {
                let new_offset = original_fat_offset;
                (
                    new_offset,
                    (align_to(new_offset + source_size) - new_offset - source_size) as u32,
                    align_to(new_offset + source_size),
                )
            };

            pak.seek(SeekFrom::Start(new_offset))
                .context("failed to seek replacement append slot")?;
            let outcome = copier.copy_file_to_writer_with_md5(&self.args.source_file, &mut pak)?;
            entries.push(
                ArchiveEntry::builder(pak_path.as_str())
                    .offset(new_offset)
                    .size(outcome.bytes())
                    .size_duplicate(outcome.bytes())
                    .padding_size(padding)
                    .md5(outcome.md5())
                    .create_time(old_entry.create_time())
                    .modify_time(modify_time)
                    .build()?,
            );
            fat_offset
        };

        let final_len =
            ArchiveWriter::xl_games().write_to(&mut pak, new_fat_offset, &entries, &extras)?;
        pak.set_len(final_len)
            .with_context(|| format!("failed to set length on {}", self.args.pak.display()))?;
        println!(
            "replaced {} in {} ({} bytes)",
            pak_path.as_str(),
            self.args.pak.display(),
            source_size
        );
        Ok(())
    }
}

fn align_to(value: u64) -> u64 {
    let remainder = value % BLOCK_SIZE;
    if remainder == 0 {
        value
    } else {
        value + (BLOCK_SIZE - remainder)
    }
}
