use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use archeage_pak::{
    cli::Cli,
    commands::CommandRunner,
    pak::{Archive, PakPath},
};
use clap::Parser;
use tempfile::tempdir;

#[test]
fn create_extract_replace_and_extract_all_roundtrip() -> Result<()> {
    let temp = tempdir()?;
    let source_dir = temp.path().join("source");
    let nested_dir = source_dir.join("nested");
    fs::create_dir_all(&nested_dir)?;
    fs::write(source_dir.join("alpha.txt"), b"alpha")?;
    fs::write(
        nested_dir.join("beta.bin"),
        (0_u8..=255).collect::<Vec<_>>(),
    )?;

    let pak = temp.path().join("roundtrip_pak");
    run_cli(["create".into(), path_arg(&source_dir), path_arg(&pak)])?;

    let archive = Archive::open(&pak)?;
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.find("alpha.txt").is_some());
    assert!(archive.find("nested/beta.bin").is_some());

    let single_out = temp.path().join("single_alpha.txt");
    run_cli([
        "extract-file".into(),
        path_arg(&pak),
        OsString::from("alpha.txt"),
        path_arg(&single_out),
    ])?;
    assert_eq!(fs::read(&single_out)?, b"alpha");

    let replacement = temp.path().join("replacement.bin");
    let replacement_bytes = (0..1500)
        .map(|value| (value % 251) as u8)
        .collect::<Vec<_>>();
    fs::write(&replacement, &replacement_bytes)?;
    run_cli([
        "replace".into(),
        path_arg(&pak),
        OsString::from("alpha.txt"),
        path_arg(&replacement),
    ])?;

    let replaced_archive = Archive::open(&pak)?;
    let replaced_entry = replaced_archive.find("alpha.txt").expect("replaced entry");
    assert_eq!(replaced_entry.size(), replacement_bytes.len() as u64);
    assert_eq!(replaced_archive.extras().len(), 1);

    let replaced_out = temp.path().join("replaced_alpha.bin");
    run_cli([
        "extract-file".into(),
        path_arg(&pak),
        OsString::from("alpha.txt"),
        path_arg(&replaced_out),
    ])?;
    assert_eq!(fs::read(&replaced_out)?, replacement_bytes);

    let extract_all_dir = temp.path().join("all");
    run_cli([
        "extract-all".into(),
        path_arg(&pak),
        path_arg(&extract_all_dir),
        "--jobs".into(),
        "2".into(),
    ])?;
    assert_eq!(
        fs::read(extract_all_dir.join("alpha.txt"))?,
        replacement_bytes
    );
    assert_eq!(
        fs::read(extract_all_dir.join("nested").join("beta.bin"))?,
        (0_u8..=255).collect::<Vec<_>>()
    );

    Ok(())
}

#[test]
fn pak_paths_reject_traversal() {
    assert!(PakPath::new("../x").is_err());
    assert!(PakPath::new("x/../y").is_err());
    assert!(PakPath::new("/absolute/is/normalized").is_ok());
}

fn run_cli<const N: usize>(args: [OsString; N]) -> Result<()> {
    let args = std::iter::once(OsString::from("archeage-pak")).chain(args);
    CommandRunner::new().run(Cli::parse_from(args))
}

fn path_arg(path: &Path) -> OsString {
    PathBuf::from(path).into_os_string()
}
