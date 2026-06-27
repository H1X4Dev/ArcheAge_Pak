use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct ReplaceArgs {
    /// Pak file to edit in place.
    pub pak: PathBuf,

    /// File name inside the pak, using forward slashes.
    pub pak_path: String,

    /// Replacement file on disk.
    pub source_file: PathBuf,

    /// Fail if the replacement does not fit in the current slot.
    #[arg(long)]
    pub in_place_only: bool,
}
