use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Source directory to pack.
    pub source_dir: PathBuf,

    /// Output pak path. Existing file is overwritten.
    pub pak: PathBuf,

    /// Optional pak path prefix for every source file.
    #[arg(short, long)]
    pub prefix: Option<String>,
}
