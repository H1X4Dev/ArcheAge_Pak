use std::fs::File;

use anyhow::{Context, Result, ensure};

use crate::{
    cli::ApplyPatchArgs,
    io::StreamCopier,
    pak::{Archive, ArchiveMutator, DELETED_MANIFEST_PATH, DeletedManifest, PakPath},
};

pub struct ApplyPatchCommand {
    args: ApplyPatchArgs,
}

impl ApplyPatchCommand {
    pub fn new(args: ApplyPatchArgs) -> Self {
        Self { args }
    }

    pub fn execute(&self) -> Result<()> {
        let source_path = self
            .args
            .source
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", self.args.source.display()))?;
        let target_path = self
            .args
            .target
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", self.args.target.display()))?;
        ensure!(
            source_path != target_path,
            "source and target must be different files"
        );

        let source = Archive::open(&source_path)?;
        let target_archive = Archive::open(&target_path)?;
        let existing_deleted =
            Self::read_entry_bytes(&target_path, target_archive.find(DELETED_MANIFEST_PATH))?;

        let mut source_file = File::open(&source_path)
            .with_context(|| format!("failed to open {}", source_path.display()))?;
        let mut target = ArchiveMutator::open(&target_path)?;
        let allow_append = !self.args.in_place_only;
        let copier = StreamCopier::default_large();

        let mut copied = 0_usize;
        let mut replaced = 0_usize;

        for entry in source.entries() {
            if entry.name() == DELETED_MANIFEST_PATH {
                continue;
            }
            let pak_path = PakPath::new(entry.name())?;
            if target.upsert_from_entry(&mut source_file, entry, &pak_path, allow_append)? {
                replaced += 1;
            } else {
                copied += 1;
            }
        }

        let mut removed = 0_usize;
        let mut missing_deletes = 0_usize;
        let mut merged_deleted = false;

        if let Some(deleted_entry) = source.find(DELETED_MANIFEST_PATH) {
            let source_deleted = copier.copy_range_to_vec(
                &mut source_file,
                deleted_entry.offset(),
                deleted_entry.size(),
            )?;
            let delete_paths = DeletedManifest::parse(&source_deleted)?;
            for path in &delete_paths {
                if target.remove_file(path)? {
                    removed += 1;
                } else {
                    missing_deletes += 1;
                    eprintln!("warning: delete path not in target: {}", path.as_str());
                }
            }

            let merged = DeletedManifest::merge(existing_deleted.as_deref(), &source_deleted);
            let deleted_path = PakPath::new(DELETED_MANIFEST_PATH)?;
            if target.upsert_bytes(&merged, &deleted_path, allow_append)? {
                replaced += 1;
            } else {
                copied += 1;
            }
            merged_deleted = true;
        }

        let final_len = target.finish()?;
        println!(
            "patched {} from {}: copied {}, replaced {}, removed {}, missing deletes {}, merged deleted.txt {}, final size {} bytes",
            target_path.display(),
            source_path.display(),
            copied,
            replaced,
            removed,
            missing_deletes,
            merged_deleted,
            final_len
        );
        Ok(())
    }

    fn read_entry_bytes(
        archive_path: &std::path::Path,
        entry: Option<&crate::pak::ArchiveEntry>,
    ) -> Result<Option<Vec<u8>>> {
        let Some(entry) = entry else {
            return Ok(None);
        };
        let mut file = File::open(archive_path)
            .with_context(|| format!("failed to open {}", archive_path.display()))?;
        let bytes = StreamCopier::default_large().copy_range_to_vec(
            &mut file,
            entry.offset(),
            entry.size(),
        )?;
        Ok(Some(bytes))
    }
}
