use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::{
    cli::AddArgs,
    io::DirectorySource,
    pak::{ArchiveMutator, PakPath},
};

pub struct AddCommand {
    args: AddArgs,
}

impl AddCommand {
    pub fn new(args: AddArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let mut mutator = ArchiveMutator::open(&self.args.pak)?;
        let (added, replaced) = if self.args.source.is_dir() {
            self.add_directory(&mut mutator)?
        } else if self.args.source.is_file() {
            self.add_file(&mut mutator)?
        } else {
            bail!("source does not exist: {}", self.args.source.display());
        };
        let final_len = mutator.finish()?;
        println!(
            "updated {}: added {}, replaced {}, final size {} bytes",
            self.args.pak.display(),
            added,
            replaced,
            final_len
        );
        Ok(())
    }

    fn add_directory(&self, mutator: &mut ArchiveMutator) -> Result<(usize, usize)> {
        let source = DirectorySource::new(&self.args.source)?;
        let files = source.files()?;
        let mut added = 0_usize;
        let mut replaced = 0_usize;
        for file_path in &files {
            let relative = file_path
                .strip_prefix(source.root())
                .with_context(|| format!("failed to relativize {}", file_path.display()))?;
            let pak_path = PakPath::from_disk_relative(relative, self.args.target.as_deref())?;
            if mutator.upsert_file(file_path, &pak_path, !self.args.in_place_only)? {
                replaced += 1;
            } else {
                added += 1;
            }
        }
        Ok((added, replaced))
    }

    fn add_file(&self, mutator: &mut ArchiveMutator) -> Result<(usize, usize)> {
        let pak_path = self.target_file_path()?;
        let replaced =
            mutator.upsert_file(&self.args.source, &pak_path, !self.args.in_place_only)?;
        if replaced { Ok((0, 1)) } else { Ok((1, 0)) }
    }

    fn target_file_path(&self) -> Result<PakPath> {
        if let Some(target) = &self.args.target {
            return PakPath::new(target.to_string());
        }
        let file_name =
            self.args.source.file_name().with_context(|| {
                format!("source has no filename: {}", self.args.source.display())
            })?;
        PakPath::from_disk_relative(&PathBuf::from(file_name), None)
    }
}
