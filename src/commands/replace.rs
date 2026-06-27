use anyhow::{Context, Result, bail};

use crate::{
    cli::ReplaceArgs,
    pak::{ArchiveMutator, PakPath},
};

pub struct ReplaceCommand {
    args: ReplaceArgs,
}

impl ReplaceCommand {
    pub fn new(args: ReplaceArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let pak_path = PakPath::new(self.args.pak_path.clone())?;
        let source_metadata = self.args.source_file.metadata().with_context(|| {
            format!(
                "failed to stat replacement file {}",
                self.args.source_file.display()
            )
        })?;
        let source_size = source_metadata.len();
        let mut mutator = ArchiveMutator::open(&self.args.pak)?;
        if !mutator.contains_file(&pak_path) {
            bail!("file not found in pak: {}", pak_path.as_str());
        }
        mutator.upsert_file(&self.args.source_file, &pak_path, !self.args.in_place_only)?;
        mutator.finish()?;
        println!(
            "replaced {} in {} ({} bytes)",
            pak_path.as_str(),
            self.args.pak.display(),
            source_size
        );
        Ok(())
    }
}
