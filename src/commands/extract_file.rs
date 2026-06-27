use std::fs::File;

use anyhow::{Context, Result, bail};
use filetime::set_file_mtime;

use crate::{
    cli::ExtractFileArgs,
    filetime::WindowsFileTime,
    io::StreamCopier,
    pak::{Archive, PakPath},
};

pub struct ExtractFileCommand {
    args: ExtractFileArgs,
}

impl ExtractFileCommand {
    pub fn new(args: ExtractFileArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let archive = Archive::open(&self.args.pak)?;
        let pak_path = PakPath::new(self.args.pak_path.clone())?;
        let entry = archive
            .find(pak_path.as_str())
            .with_context(|| format!("file not found in pak: {}", pak_path.as_str()))?;
        let mut reader = File::open(&self.args.pak)
            .with_context(|| format!("failed to open {}", self.args.pak.display()))?;
        let copied = StreamCopier::default_large().copy_range_to_path(
            &mut reader,
            entry.offset(),
            entry.size(),
            &self.args.out_file,
        )?;
        if copied != entry.size() {
            bail!(
                "short extract for {}: copied {copied}, expected {}",
                entry.name(),
                entry.size()
            );
        }
        if let Some(mtime) = WindowsFileTime::try_to_file_time(entry.modify_time()) {
            set_file_mtime(&self.args.out_file, mtime).with_context(|| {
                format!("failed to set mtime on {}", self.args.out_file.display())
            })?;
        }
        println!(
            "extracted {} -> {} ({} bytes)",
            entry.name(),
            self.args.out_file.display(),
            copied
        );
        Ok(())
    }
}
