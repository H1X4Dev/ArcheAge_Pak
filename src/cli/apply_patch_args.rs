use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct ApplyPatchArgs {
    /// Source patch pak whose contents are copied into the target.
    pub source: PathBuf,

    /// Target pak to patch in place.
    pub target: PathBuf,

    /// Fail instead of appending when no existing/free slot can hold a file.
    #[arg(long)]
    pub in_place_only: bool,
}
