use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PakPath {
    value: String,
}

impl PakPath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let mut value = value.into().replace('\\', "/");
        while let Some(stripped) = value.strip_prefix('/') {
            value = stripped.to_string();
        }
        ensure!(!value.is_empty(), "pak path cannot be empty");
        ensure!(!value.contains('\0'), "pak path contains a NUL byte");
        for segment in value.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                bail!("unsafe pak path segment in {value}");
            }
        }
        Ok(Self { value })
    }

    pub fn from_disk_relative(path: &Path, prefix: Option<&str>) -> Result<Self> {
        let mut parts = Vec::new();
        if let Some(prefix) = prefix {
            let prefix = Self::new(prefix.to_string())?;
            parts.push(prefix.value);
        }

        for component in path.components() {
            match component {
                Component::Normal(value) => parts.push(value.to_string_lossy().replace('\\', "/")),
                Component::CurDir => {}
                _ => bail!("source path cannot be packed safely: {}", path.display()),
            }
        }

        Self::new(parts.join("/"))
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn join_to(&self, root: &Path) -> Result<PathBuf> {
        let mut out = root.to_path_buf();
        for segment in self.value.split('/') {
            out.push(Self::windows_safe_segment(segment));
        }
        Ok(out)
    }

    fn windows_safe_segment(segment: &str) -> String {
        let mut output = String::with_capacity(segment.len());
        for (index, character) in segment.char_indices() {
            let is_trailing = index + character.len_utf8() == segment.len();
            if Self::must_escape_windows_character(character, is_trailing) {
                Self::push_percent_encoded_character(&mut output, character);
            } else {
                output.push(character);
            }
        }

        if Self::is_reserved_windows_name(&output) {
            output.push_str("%00");
        }

        output
    }

    fn must_escape_windows_character(character: char, is_trailing: bool) -> bool {
        matches!(
            character,
            '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' | '%'
        ) || character.is_control()
            || (is_trailing && matches!(character, ' ' | '.'))
    }

    fn push_percent_encoded_character(output: &mut String, character: char) {
        let mut bytes = [0_u8; 4];
        for byte in character.encode_utf8(&mut bytes).as_bytes() {
            output.push('%');
            output.push_str(&format!("{byte:02X}"));
        }
    }

    fn is_reserved_windows_name(segment: &str) -> bool {
        let name = segment.split('.').next().unwrap_or(segment);
        matches!(
            name.to_ascii_uppercase().as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
    }
}
