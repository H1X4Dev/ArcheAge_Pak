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
fn create_extract_replace_add_and_extract_all_roundtrip() -> Result<()> {
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

    let missing_replace = run_cli([
        "replace".into(),
        path_arg(&pak),
        OsString::from("missing.bin"),
        path_arg(&replacement),
    ]);
    assert!(missing_replace.is_err());

    let add_dir = temp.path().join("add_source");
    let add_nested_dir = add_dir.join("nested");
    fs::create_dir_all(&add_nested_dir)?;
    let beta_update = (0..600)
        .map(|value| 255_u8.wrapping_sub((value % 251) as u8))
        .collect::<Vec<_>>();
    let gamma_bytes = b"new file from directory add".to_vec();
    fs::write(add_nested_dir.join("beta.bin"), &beta_update)?;
    fs::write(add_dir.join("gamma.txt"), &gamma_bytes)?;
    run_cli(["add".into(), path_arg(&pak), path_arg(&add_dir)])?;

    let loose = temp.path().join("loose.dat");
    let loose_bytes = b"single file add".to_vec();
    fs::write(&loose, &loose_bytes)?;
    run_cli([
        "add".into(),
        path_arg(&pak),
        path_arg(&loose),
        OsString::from("extra/loose.dat"),
    ])?;

    let added_archive = Archive::open(&pak)?;
    assert_eq!(added_archive.entries().len(), 4);
    assert_eq!(
        added_archive
            .find("nested/beta.bin")
            .expect("updated beta")
            .size(),
        beta_update.len() as u64
    );
    assert!(added_archive.find("gamma.txt").is_some());
    assert!(added_archive.find("extra/loose.dat").is_some());

    let added_extract_dir = temp.path().join("added_all");
    run_cli([
        "extract-all".into(),
        path_arg(&pak),
        path_arg(&added_extract_dir),
        "--jobs".into(),
        "2".into(),
    ])?;
    assert_eq!(
        fs::read(added_extract_dir.join("nested").join("beta.bin"))?,
        beta_update
    );
    assert_eq!(fs::read(added_extract_dir.join("gamma.txt"))?, gamma_bytes);
    assert_eq!(
        fs::read(added_extract_dir.join("extra").join("loose.dat"))?,
        loose_bytes
    );

    Ok(())
}

#[test]
fn add_normalizes_import_paths_to_existing_archive_casing() -> Result<()> {
    let temp = tempdir()?;
    let source_dir = temp.path().join("source");
    let existing_dir = source_dir.join("game").join("libs").join("particles");
    fs::create_dir_all(&existing_dir)?;
    fs::write(existing_dir.join("existing.xml"), b"old")?;

    let pak = temp.path().join("case_normalized_pak");
    run_cli(["create".into(), path_arg(&source_dir), path_arg(&pak)])?;

    let patch_dir = temp.path().join("patch");
    let patch_particles = patch_dir.join("game").join("Libs").join("Particles");
    fs::create_dir_all(&patch_particles)?;
    fs::write(patch_particles.join("existing.xml"), b"new")?;
    fs::write(patch_particles.join("fresh.xml"), b"fresh")?;
    run_cli(["add".into(), path_arg(&pak), path_arg(&patch_dir)])?;

    let archive = Archive::open(&pak)?;
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.find("game/libs/particles/existing.xml").is_some());
    assert!(archive.find("game/libs/particles/fresh.xml").is_some());
    assert!(archive.find("game/Libs/Particles/existing.xml").is_none());
    assert!(archive.find("game/Libs/Particles/fresh.xml").is_none());

    let existing_out = temp.path().join("existing.xml");
    run_cli([
        "extract-file".into(),
        path_arg(&pak),
        OsString::from("game/libs/particles/existing.xml"),
        path_arg(&existing_out),
    ])?;
    assert_eq!(fs::read(&existing_out)?, b"new");

    let fresh_out = temp.path().join("fresh.xml");
    run_cli([
        "extract-file".into(),
        path_arg(&pak),
        OsString::from("game/libs/particles/fresh.xml"),
        path_arg(&fresh_out),
    ])?;
    assert_eq!(fs::read(&fresh_out)?, b"fresh");

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
