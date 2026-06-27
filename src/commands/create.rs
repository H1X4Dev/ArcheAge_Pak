use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
};

use anyhow::{Context, Result};

use crate::{
    cli::CreateArgs,
    filetime::WindowsFileTime,
    io::{DirectorySource, StreamCopier},
    pak::{ArchiveEntry, ArchiveWriter, BlockAlignment, PakPath},
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
            let metadata = file_path
                .metadata()
                .with_context(|| format!("failed to stat {}", file_path.display()))?;
            let create_time = WindowsFileTime::from_system_time(
                metadata.created().or_else(|_| metadata.modified())?,
            )
            .value();
            let modify_time = WindowsFileTime::from_system_time(metadata.modified()?).value();
            let outcome = copier.copy_file_to_writer_with_md5(file_path, &mut pak)?;
            let padding = BlockAlignment::padding_for_size(outcome.bytes()) as u32;
            if padding > 0 {
                write_zeros(&mut pak, padding as usize)?;
            }
            entries.push(
                ArchiveEntry::builder(pak_path.as_str())
                    .offset(offset)
                    .size(outcome.bytes())
                    .size_duplicate(outcome.bytes())
                    .padding_size(padding)
                    .md5(outcome.md5())
                    .create_time(create_time)
                    .modify_time(modify_time)
                    .build()?,
            );
            offset += outcome.bytes() + u64::from(padding);
        }

        let final_len = ArchiveWriter::xl_games().write_to(&mut pak, offset, &entries, &[])?;
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

fn write_zeros(writer: &mut File, mut bytes: usize) -> Result<()> {
    const ZEROES: [u8; 8192] = [0; 8192];
    while bytes > 0 {
        let chunk = bytes.min(ZEROES.len());
        writer
            .write_all(&ZEROES[..chunk])
            .context("failed to write pak payload padding")?;
        bytes -= chunk;
    }
    Ok(())
}
