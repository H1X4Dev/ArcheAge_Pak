use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Pak file to inspect.
    pub pak: PathBuf,

    /// Optional case-sensitive substring filter.
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Stop after printing this many matching entries.
    #[arg(short, long)]
    pub limit: Option<usize>,
}
