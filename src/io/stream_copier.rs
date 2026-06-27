use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{Context, Result};
use md5::{Digest, Md5};

pub struct CopyOutcome {
    bytes: u64,
    md5: [u8; 16],
}

impl CopyOutcome {
    pub fn new(bytes: u64, md5: [u8; 16]) -> Self {
        Self { bytes, md5 }
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn md5(&self) -> [u8; 16] {
        self.md5
    }
}

pub struct StreamCopier {
    buffer_size: usize,
}

impl StreamCopier {
    pub fn new(buffer_size: usize) -> Self {
        Self { buffer_size }
    }

    pub fn default_large() -> Self {
        Self::new(8 * 1024 * 1024)
    }

    pub fn copy_file_to_writer_with_md5(
        &self,
        source_path: &Path,
        writer: &mut File,
    ) -> Result<CopyOutcome> {
        let mut source = File::open(source_path)
            .with_context(|| format!("failed to open source {}", source_path.display()))?;
        let mut buffer = vec![0_u8; self.buffer_size];
        let mut hasher = Md5::new();
        let mut bytes = 0_u64;

        loop {
            let read = source
                .read(&mut buffer)
                .with_context(|| format!("failed to read {}", source_path.display()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            writer
                .write_all(&buffer[..read])
                .context("failed to write pak payload")?;
            bytes += read as u64;
        }

        Ok(CopyOutcome::new(bytes, hasher.finalize().into()))
    }

    pub fn copy_range_to_path(
        &self,
        reader: &mut File,
        offset: u64,
        size: u64,
        out_path: &Path,
    ) -> Result<u64> {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        reader
            .seek(SeekFrom::Start(offset))
            .context("failed to seek to pak payload")?;
        let mut output = File::create(out_path)
            .with_context(|| format!("failed to create {}", out_path.display()))?;
        let mut limited = reader.take(size);
        let copied = std::io::copy(&mut limited, &mut output)
            .with_context(|| format!("failed to extract {}", out_path.display()))?;
        Ok(copied)
    }
}
