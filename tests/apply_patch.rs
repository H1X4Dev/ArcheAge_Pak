use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use archeage_pak::{
    cli::Cli,
    commands::CommandRunner,
    pak::{Archive, DeletedManifest, PakPath},
};
use clap::Parser;
use tempfile::tempdir;

#[test]
fn apply_patch_copies_and_deletes() -> Result<()> {
    let temp = tempdir()?;

    let target_dir = temp.path().join("target_source");
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("keep.txt"), b"original keep")?;
    fs::write(target_dir.join("remove.txt"), b"to be removed")?;
    fs::write(target_dir.join("deleted.txt"), b"/game/old/path.cgf\n")?;

    let target_pak = temp.path().join("target.pak");
    run_cli([
        "create".into(),
        path_arg(&target_dir),
        path_arg(&target_pak),
    ])?;

    let source_dir = temp.path().join("patch_source");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("keep.txt"), b"patched keep")?;
    fs::write(source_dir.join("new.txt"), b"brand new")?;
    fs::write(source_dir.join("deleted.txt"), b"/remove.txt\n")?;

    let source_pak = temp.path().join("source.pak");
    run_cli([
        "create".into(),
        path_arg(&source_dir),
        path_arg(&source_pak),
    ])?;

    run_cli([
        "apply-patch".into(),
        path_arg(&source_pak),
        path_arg(&target_pak),
    ])?;

    let archive = Archive::open(&target_pak)?;
    assert_eq!(archive.entries().len(), 3);
    assert!(archive.find("remove.txt").is_none());
    assert!(archive.find("new.txt").is_some());
    assert_eq!(
        read_pak_entry(&target_pak, "keep.txt")?,
        b"patched keep".as_slice()
    );
    assert_eq!(
        read_pak_entry(&target_pak, "new.txt")?,
        b"brand new".as_slice()
    );

    let deleted = read_pak_entry(&target_pak, "deleted.txt")?;
    assert_eq!(deleted, b"/game/old/path.cgf\n/remove.txt\n".as_slice());

    Ok(())
}

#[test]
fn apply_patch_without_deleted_txt() -> Result<()> {
    let temp = tempdir()?;

    let target_dir = temp.path().join("target_source");
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("existing.txt"), b"existing")?;

    let target_pak = temp.path().join("target.pak");
    run_cli([
        "create".into(),
        path_arg(&target_dir),
        path_arg(&target_pak),
    ])?;

    let source_dir = temp.path().join("patch_source");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("existing.txt"), b"updated")?;
    fs::write(source_dir.join("added.txt"), b"added")?;

    let source_pak = temp.path().join("source.pak");
    run_cli([
        "create".into(),
        path_arg(&source_dir),
        path_arg(&source_pak),
    ])?;

    run_cli([
        "apply-patch".into(),
        path_arg(&source_pak),
        path_arg(&target_pak),
    ])?;

    let archive = Archive::open(&target_pak)?;
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.find("deleted.txt").is_none());
    assert_eq!(
        read_pak_entry(&target_pak, "existing.txt")?,
        b"updated".as_slice()
    );
    assert_eq!(
        read_pak_entry(&target_pak, "added.txt")?,
        b"added".as_slice()
    );

    Ok(())
}

#[test]
fn apply_patch_warns_on_missing_delete() -> Result<()> {
    let temp = tempdir()?;

    let target_dir = temp.path().join("target_source");
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("keep.txt"), b"keep")?;

    let target_pak = temp.path().join("target.pak");
    run_cli([
        "create".into(),
        path_arg(&target_dir),
        path_arg(&target_pak),
    ])?;

    let source_dir = temp.path().join("patch_source");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("deleted.txt"), b"/missing/path.cgf\n")?;

    let source_pak = temp.path().join("source.pak");
    run_cli([
        "create".into(),
        path_arg(&source_dir),
        path_arg(&source_pak),
    ])?;

    run_cli([
        "apply-patch".into(),
        path_arg(&source_pak),
        path_arg(&target_pak),
    ])?;

    let archive = Archive::open(&target_pak)?;
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.find("keep.txt").is_some());
    assert_eq!(
        read_pak_entry(&target_pak, "deleted.txt")?,
        b"/missing/path.cgf\n".as_slice()
    );

    Ok(())
}

#[test]
fn apply_patch_deletes_case_variant_target_entry() -> Result<()> {
    let temp = tempdir()?;

    let target_dir = temp.path().join("target_source");
    let target_nested = target_dir.join("Game").join("Libs");
    fs::create_dir_all(&target_nested)?;
    fs::write(target_nested.join("Foo.txt"), b"remove me")?;

    let target_pak = temp.path().join("target.pak");
    run_cli([
        "create".into(),
        path_arg(&target_dir),
        path_arg(&target_pak),
    ])?;

    let source_dir = temp.path().join("patch_source");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("deleted.txt"), b"/game/libs/foo.txt\n")?;

    let source_pak = temp.path().join("source.pak");
    run_cli([
        "create".into(),
        path_arg(&source_dir),
        path_arg(&source_pak),
    ])?;

    run_cli([
        "apply-patch".into(),
        path_arg(&source_pak),
        path_arg(&target_pak),
    ])?;

    let archive = Archive::open(&target_pak)?;
    assert_eq!(archive.entries().len(), 1);
    assert!(archive.find("Game/Libs/Foo.txt").is_none());
    assert_eq!(
        read_pak_entry(&target_pak, "deleted.txt")?,
        b"/game/libs/foo.txt\n".as_slice()
    );

    Ok(())
}

#[test]
fn apply_patch_normalizes_source_paths_to_existing_archive_casing() -> Result<()> {
    let temp = tempdir()?;

    let target_dir = temp.path().join("target_source");
    let target_particles = target_dir.join("game").join("libs").join("particles");
    fs::create_dir_all(&target_particles)?;
    fs::write(target_particles.join("existing.xml"), b"old")?;

    let target_pak = temp.path().join("target.pak");
    run_cli([
        "create".into(),
        path_arg(&target_dir),
        path_arg(&target_pak),
    ])?;

    let source_dir = temp.path().join("patch_source");
    let source_particles = source_dir.join("game").join("Libs").join("Particles");
    fs::create_dir_all(&source_particles)?;
    fs::write(source_particles.join("existing.xml"), b"new")?;
    fs::write(source_particles.join("fresh.xml"), b"fresh")?;

    let source_pak = temp.path().join("source.pak");
    run_cli([
        "create".into(),
        path_arg(&source_dir),
        path_arg(&source_pak),
    ])?;

    run_cli([
        "apply-patch".into(),
        path_arg(&source_pak),
        path_arg(&target_pak),
    ])?;

    let archive = Archive::open(&target_pak)?;
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.find("game/libs/particles/existing.xml").is_some());
    assert!(archive.find("game/libs/particles/fresh.xml").is_some());
    assert!(archive.find("game/Libs/Particles/existing.xml").is_none());
    assert!(archive.find("game/Libs/Particles/fresh.xml").is_none());
    assert_eq!(
        read_pak_entry(&target_pak, "game/libs/particles/existing.xml")?,
        b"new".as_slice()
    );
    assert_eq!(
        read_pak_entry(&target_pak, "game/libs/particles/fresh.xml")?,
        b"fresh".as_slice()
    );

    Ok(())
}

#[test]
fn deleted_manifest_parses_leading_slash() -> Result<()> {
    let paths = DeletedManifest::parse(b"/game/foo.cgf\n/game/bar.cgf\n")?;
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0], PakPath::new("game/foo.cgf")?);
    assert_eq!(paths[1], PakPath::new("game/bar.cgf")?);
    Ok(())
}

#[test]
fn deleted_manifest_merge_appends_with_separator() {
    let merged = DeletedManifest::merge(Some(b"line1"), b"line2\n");
    assert_eq!(merged, b"line1\nline2\n");

    let merged = DeletedManifest::merge(None, b"line2\n");
    assert_eq!(merged, b"line2\n");
}

#[test]
fn apply_patch_rejects_same_file() -> Result<()> {
    let temp = tempdir()?;
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("alpha.txt"), b"alpha")?;

    let pak = temp.path().join("same.pak");
    run_cli(["create".into(), path_arg(&source_dir), path_arg(&pak)])?;

    let result = run_cli(["apply-patch".into(), path_arg(&pak), path_arg(&pak)]);
    assert!(result.is_err());

    Ok(())
}

fn read_pak_entry(pak: &Path, pak_path: &str) -> Result<Vec<u8>> {
    let archive = Archive::open(pak)?;
    let entry = archive
        .find(pak_path)
        .with_context(|| format!("file not found in pak: {pak_path}"))?;
    let mut file = fs::File::open(pak)?;
    archeage_pak::io::StreamCopier::default_large().copy_range_to_vec(
        &mut file,
        entry.offset(),
        entry.size(),
    )
}

fn run_cli<const N: usize>(args: [OsString; N]) -> Result<()> {
    let args = std::iter::once(OsString::from("archeage-pak")).chain(args);
    CommandRunner::new().run(Cli::parse_from(args))
}

fn path_arg(path: &Path) -> OsString {
    PathBuf::from(path).into_os_string()
}
