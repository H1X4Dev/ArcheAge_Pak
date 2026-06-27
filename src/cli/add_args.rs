use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Pak file to edit in place.
    pub pak: PathBuf,

    /// Source file or directory to add.
    pub source: PathBuf,

    /// Target pak path for a file, or target pak prefix for a directory.
    pub target: Option<String>,

    /// Fail instead of appending when no existing/free slot can hold the file.
    #[arg(long)]
    pub in_place_only: bool,
}
