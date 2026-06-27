use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct ExtractFileArgs {
    /// Pak file to read.
    pub pak: PathBuf,

    /// File name inside the pak, using forward slashes.
    pub pak_path: String,

    /// Destination file path.
    pub out_file: PathBuf,
}
