# ArcheAge Pak

Command-line tooling for ArcheAge `game_pak` archives.

`archeage-pak` can list, extract, create, and edit pak files. It is designed for fast local workflows where you need to unpack a full archive, extract one file, build a new archive from a directory, or patch an existing archive by replacing or adding files.

## Features

- List pak metadata and file names.
- Extract a single file by pak path.
- Extract all files, optionally filtered by a pak path prefix.
- Create a new pak from a directory tree.
- Replace an existing file in a pak.
- Add a file or directory to an existing pak.
- When adding a path that already exists, the existing pak entry is replaced.
- Apply a patch pak to an existing pak, including `deleted.txt` deletions.
- Reuses existing free slots when possible and appends before rewriting the FAT when needed.

## Safety

Editing commands modify pak files in place. Keep a backup of any original archive before using `add`, `replace`, or `apply-patch`.

Pak paths use forward slashes, for example `game/config/example.xml`. Unsafe paths such as `../file` are rejected.

## Install

Build a release binary with Rust 1.91 or newer:

```powershell
cargo build --release
```

The binary is written to:

```text
target\release\archeage-pak.exe
```

## Usage

```text
archeage-pak.exe <COMMAND>
```

Commands:

```text
list          Print archive metadata and file names
extract-all   Extract every file, or every file under a pak path prefix
extract-file  Extract one file from the pak
create        Create a new pak from a directory
add           Add or replace a file/directory in an existing pak
replace       Replace one file in an existing pak
apply-patch   Copy a source pak into a target pak and apply deleted.txt deletions
```

## Examples

List archive contents:

```powershell
archeage-pak.exe list F:\path\to\game_pak
```

Extract one file:

```powershell
archeage-pak.exe extract-file F:\path\to\game_pak game/config/example.xml F:\out\example.xml
```

Extract the full archive:

```powershell
archeage-pak.exe extract-all F:\path\to\game_pak F:\out\game_pak
```

Extract only a prefix:

```powershell
archeage-pak.exe extract-all F:\path\to\game_pak F:\out\ui --prefix game/ui
```

Create a new pak from a directory:

```powershell
archeage-pak.exe create F:\input\directory F:\out\new_game_pak
```

Create a new pak while placing every file under a pak prefix:

```powershell
archeage-pak.exe create F:\input\directory F:\out\new_game_pak --prefix game/custom
```

Replace one existing file:

```powershell
archeage-pak.exe replace F:\path\to\game_pak game/config/example.xml F:\patch\example.xml
```

Add one file at an explicit pak path. If the pak path already exists, it is replaced:

```powershell
archeage-pak.exe add F:\path\to\game_pak F:\patch\example.xml game/config/example.xml
```

Add a directory at the pak root. Directory contents keep their relative paths, and existing files are replaced:

```powershell
archeage-pak.exe add F:\path\to\game_pak F:\patch_directory
```

Add a directory under a pak prefix:

```powershell
archeage-pak.exe add F:\path\to\game_pak F:\patch_directory game/custom
```

Fail instead of appending if the new payload cannot fit into an existing slot:

```powershell
archeage-pak.exe add F:\path\to\game_pak F:\patch_directory --in-place-only
archeage-pak.exe replace F:\path\to\game_pak game/config/example.xml F:\patch\example.xml --in-place-only
```

Apply a patch pak to an existing pak. All source files except `deleted.txt` are copied into the target. If the source contains `deleted.txt`, each listed path is removed from the target and the source manifest is appended to the target's `deleted.txt`:

```powershell
archeage-pak.exe apply-patch F:\patch.pak F:\path\to\game_pak
```

## Development

Run the standard checks:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Build the optimized binary:

```powershell
cargo build --release
```

## Notes

`add` is an upsert operation:

- Existing pak path: replace the entry.
- New pak path: insert a new entry.
- Directory source: apply that rule to every file in the directory tree.

`replace` is strict:

- Existing pak path: replace the entry.
- Missing pak path: return an error.
