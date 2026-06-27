use std::path::PathBuf;

use clap::Args;

use super::FormatArgs;

#[derive(Debug, Args)]
pub struct CreateArgs {
    #[command(flatten)]
    pub format: FormatArgs,

    /// Source directory to pack.
    pub source_dir: PathBuf,

    /// Output pak path. Existing file is overwritten.
    pub pak: PathBuf,

    /// Optional pak path prefix for every source file.
    #[arg(short, long)]
    pub prefix: Option<String>,
}
