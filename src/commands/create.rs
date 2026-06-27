use std::{
    fs::{self, File},
    io::{Seek, SeekFrom},
};

use anyhow::{Context, Result};

use crate::{
    cli::CreateArgs,
    io::{DirectorySource, StreamCopier},
    pak::{
        ArchiveEntry, ArchiveEntryPayload, ArchiveFileMetadata, ArchiveWriter, BlockAlignment,
        PakPath,
    },
};

pub struct CreateCommand {
    args: CreateArgs,
}

impl CreateCommand {
    pub fn new(args: CreateArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let source = DirectorySource::new(&self.args.source_dir)?;
        let files = source.files()?;
        if let Some(parent) = self.args.pak.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let mut pak = File::create(&self.args.pak)
            .with_context(|| format!("failed to create {}", self.args.pak.display()))?;
        let copier = StreamCopier::default_large();
        let mut entries = Vec::with_capacity(files.len());
        let mut offset = 0_u64;

        for file_path in &files {
            pak.seek(SeekFrom::Start(offset))
                .context("failed to seek output pak")?;
            let relative = file_path
                .strip_prefix(source.root())
                .with_context(|| format!("failed to relativize {}", file_path.display()))?;
            let pak_path = PakPath::from_disk_relative(relative, self.args.prefix.as_deref())?;
            let metadata = ArchiveFileMetadata::from_path(file_path)?;
            let outcome = copier.copy_file_to_writer_with_md5(file_path, &mut pak)?;
            let padding = BlockAlignment::padding_for_size(outcome.bytes()) as u32;
            if padding > 0 {
                copier.write_zero_padding(&mut pak, padding as usize)?;
            }
            let payload = ArchiveEntryPayload::from_copy_outcome(
                offset,
                padding,
                &outcome,
                metadata.create_time(),
                metadata.modify_time(),
            );
            entries.push(ArchiveEntry::file(pak_path.as_str(), &payload)?);
            offset += outcome.bytes() + u64::from(padding);
        }

        let final_len = ArchiveWriter::for_format(self.args.format.pak_format()).write_to(
            &mut pak,
            offset,
            &entries,
            &[],
        )?;
        pak.set_len(final_len)
            .with_context(|| format!("failed to set length on {}", self.args.pak.display()))?;
        println!(
            "created {} with {} files ({} bytes)",
            self.args.pak.display(),
            entries.len(),
            final_len
        );
        Ok(())
    }
}
