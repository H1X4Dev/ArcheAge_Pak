use anyhow::Result;

use crate::{cli::ListArgs, pak::Archive};

pub struct ListCommand {
    args: ListArgs,
}

impl ListCommand {
    pub fn new(args: ListArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let archive = Archive::open(&self.args.pak)?;
        println!(
            "pak={} files={} extra={} fat_offset={} size={}",
            archive.path().display(),
            archive.header().file_count(),
            archive.header().extra_file_count(),
            archive.header().fat_offset(),
            archive.header().archive_len()
        );

        let mut printed = 0_usize;
        for entry in archive.entries() {
            if let Some(filter) = &self.args.filter
                && !entry.name().contains(filter)
            {
                continue;
            }
            println!(
                "{}\tsize={}\toffset={}\tpadding={}",
                entry.name(),
                entry.size(),
                entry.offset(),
                entry.padding_size()
            );
            printed += 1;
            if let Some(limit) = self.args.limit
                && printed >= limit
            {
                break;
            }
        }
        Ok(())
    }
}
