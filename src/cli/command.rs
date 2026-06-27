use clap::Subcommand;

use super::{
    AddArgs, ApplyPatchArgs, CreateArgs, ExtractAllArgs, ExtractFileArgs, ListArgs, ReplaceArgs,
};

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
    /// Add or replace a file/directory in an existing pak.
    Add(AddArgs),
    /// Replace one file in an existing pak.
    Replace(ReplaceArgs),
    /// Copy a source pak into a target pak and apply deleted.txt deletions.
    ApplyPatch(ApplyPatchArgs),
}
