use anyhow::Result;

use super::PakPath;

pub const DELETED_MANIFEST_PATH: &str = "deleted.txt";

pub struct DeletedManifest;

impl DeletedManifest {
    pub fn parse(content: &[u8]) -> Result<Vec<PakPath>> {
        let text = std::str::from_utf8(content)
            .map_err(|error| anyhow::anyhow!("deleted.txt is not valid UTF-8: {error}"))?;
        let mut paths = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            paths.push(PakPath::new(trimmed)?);
        }
        Ok(paths)
    }

    pub fn merge(existing: Option<&[u8]>, source: &[u8]) -> Vec<u8> {
        let Some(existing) = existing.filter(|content| !content.is_empty()) else {
            return source.to_vec();
        };

        let mut merged = existing.to_vec();
        if !merged.ends_with(b"\n") {
            merged.push(b'\n');
        }
        merged.extend_from_slice(source);
        merged
    }
}
