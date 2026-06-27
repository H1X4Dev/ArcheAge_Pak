use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::Result;
use archeage_pak::{cli::Cli, commands::CommandRunner};
use clap::Parser;

pub fn run_cli<const N: usize>(args: [OsString; N]) -> Result<()> {
    let args = std::iter::once(OsString::from("archeage-pak")).chain(args);
    CommandRunner::new().run(Cli::parse_from(args))
}

pub fn path_arg(path: &Path) -> OsString {
    PathBuf::from(path).into_os_string()
}

pub fn create_pak(source_dir: &Path, pak: &Path) -> Result<()> {
    run_cli(["create".into(), path_arg(source_dir), path_arg(pak)])
}
