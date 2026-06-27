use std::fs::File;

use anyhow::{Context, Result, anyhow, bail};
use filetime::set_file_mtime;
use rayon::prelude::*;

use crate::{
    cli::ExtractAllArgs,
    filetime::WindowsFileTime,
    io::StreamCopier,
    pak::{Archive, ArchiveEntry, PakPath},
};

pub struct ExtractAllCommand {
    args: ExtractAllArgs,
}

impl ExtractAllCommand {
    pub fn new(args: ExtractAllArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let archive = Archive::open(&self.args.pak)?;
        let entries = self.filtered_entries(&archive);
        let jobs = self.args.jobs.unwrap_or_else(num_cpus::get).max(1);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .context("failed to build extraction thread pool")?;
        let out_dir = self.args.out_dir.clone();
        let pak = self.args.pak.clone();

        pool.install(|| {
            entries.par_iter().try_for_each_init(
                || -> Result<(File, StreamCopier)> {
                    let file = File::open(&pak)
                        .with_context(|| format!("failed to open {}", pak.display()))?;
                    Ok((file, StreamCopier::default_large()))
                },
                |state, entry| {
                    let (reader, copier) = state
                        .as_mut()
                        .map_err(|error| anyhow!("extract worker init failed: {error:#}"))?;
                    let out_path = PakPath::new(entry.name().to_string())?.join_to(&out_dir)?;
                    let copied = copier.copy_range_to_path(
                        reader,
                        entry.offset(),
                        entry.size(),
                        &out_path,
                    )?;
                    if copied != entry.size() {
                        bail!(
                            "short extract for {}: copied {copied}, expected {}",
                            entry.name(),
                            entry.size()
                        );
                    }
                    let mtime = WindowsFileTime::to_file_time(entry.modify_time());
                    set_file_mtime(&out_path, mtime).with_context(|| {
                        format!("failed to set mtime on {}", out_path.display())
                    })?;
                    Ok(())
                },
            )
        })?;

        println!(
            "extracted {} files from {} to {} using {} jobs",
            entries.len(),
            self.args.pak.display(),
            self.args.out_dir.display(),
            jobs
        );
        Ok(())
    }

    fn filtered_entries(&self, archive: &Archive) -> Vec<ArchiveEntry> {
        archive
            .entries()
            .iter()
            .filter(|entry| {
                self.args
                    .prefix
                    .as_ref()
                    .is_none_or(|prefix| entry.name().starts_with(prefix))
            })
            .cloned()
            .collect()
    }
}
