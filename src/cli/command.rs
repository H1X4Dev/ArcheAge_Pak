use clap::Subcommand;

use super::{CreateArgs, ExtractAllArgs, ExtractFileArgs, ListArgs, ReplaceArgs};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print archive metadata and file names.
    List(ListArgs),
    /// Extract every file, or every file under a pak path prefix.
    ExtractAll(ExtractAllArgs),
    /// Extract one file from the pak.
    ExtractFile(ExtractFileArgs),
    /// Create a new pak from a directory.
    Create(CreateArgs),
    /// Replace one file in an existing pak.
    Replace(ReplaceArgs),
}
