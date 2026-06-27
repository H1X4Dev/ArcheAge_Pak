use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct ExtractAllArgs {
    /// Pak file to extract.
    pub pak: PathBuf,

    /// Output directory.
    pub out_dir: PathBuf,

    /// Only extract files whose pak path starts with this prefix.
    #[arg(short, long)]
    pub prefix: Option<String>,

    /// Number of parallel extraction workers. Defaults to logical CPUs.
    #[arg(short, long)]
    pub jobs: Option<usize>,
}
