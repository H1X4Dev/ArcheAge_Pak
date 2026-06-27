use std::{
    fs::{self, File},
    io::{Cursor, Read, Seek, SeekFrom, Write},
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
        self.copy_reader_to_writer_with_md5(&mut source, writer, || {
            format!("failed to read {}", source_path.display())
        })
    }

    pub fn copy_range_to_writer_with_md5(
        &self,
        reader: &mut File,
        offset: u64,
        size: u64,
        writer: &mut File,
    ) -> Result<CopyOutcome> {
        reader
            .seek(SeekFrom::Start(offset))
            .context("failed to seek to pak payload")?;
        let mut limited = reader.take(size);
        self.copy_reader_to_writer_with_md5(&mut limited, writer, || {
            "failed to read pak payload".to_string()
        })
    }

    pub fn copy_range_to_vec(&self, reader: &mut File, offset: u64, size: u64) -> Result<Vec<u8>> {
        reader
            .seek(SeekFrom::Start(offset))
            .context("failed to seek to pak payload")?;
        let mut limited = reader.take(size);
        let mut output = Vec::with_capacity(size as usize);
        std::io::copy(&mut limited, &mut output).context("failed to read pak payload")?;
        Ok(output)
    }

    pub fn copy_bytes_to_writer_with_md5(
        &self,
        data: &[u8],
        writer: &mut File,
    ) -> Result<CopyOutcome> {
        let mut reader = Cursor::new(data);
        self.copy_reader_to_writer_with_md5(&mut reader, writer, || {
            "failed to read in-memory pak payload".to_string()
        })
    }

    pub fn write_zero_padding<W>(&self, writer: &mut W, mut bytes: usize) -> Result<()>
    where
        W: Write,
    {
        const ZEROES: [u8; 8192] = [0; 8192];
        while bytes > 0 {
            let chunk = bytes.min(ZEROES.len());
            writer
                .write_all(&ZEROES[..chunk])
                .context("failed to write pak payload padding")?;
            bytes -= chunk;
        }
        Ok(())
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

    fn copy_reader_to_writer_with_md5<R, W, F>(
        &self,
        reader: &mut R,
        writer: &mut W,
        read_context: F,
    ) -> Result<CopyOutcome>
    where
        R: Read,
        W: Write,
        F: Fn() -> String,
    {
        let mut buffer = vec![0_u8; self.buffer_size];
        let mut hasher = Md5::new();
        let mut bytes = 0_u64;

        loop {
            let read = reader.read(&mut buffer).with_context(&read_context)?;
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
}
