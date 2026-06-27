use std::io::{Read, Seek, SeekFrom};

use anyhow::{Context, Result, bail, ensure};

use super::{BLOCK_SIZE, FOOTER_SIZE, PakFormat, RECORD_SIZE};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    format: PakFormat,
    file_count: usize,
    extra_file_count: usize,
    fat_offset: u64,
    archive_len: u64,
}

impl Header {
    pub fn new(
        format: PakFormat,
        file_count: usize,
        extra_file_count: usize,
        fat_offset: u64,
        archive_len: u64,
    ) -> Self {
        Self {
            format,
            file_count,
            extra_file_count,
            fat_offset,
            archive_len,
        }
    }

    pub fn read_from<R>(reader: &mut R, archive_len: u64) -> Result<Self>
    where
        R: Read + Seek,
    {
        let encrypted = Self::read_encrypted_footer(reader, archive_len)?;
        for format in PakFormat::known() {
            if let Some(header) = Self::try_parse_footer(encrypted, archive_len, *format)? {
                return Ok(header);
            }
        }

        bail!("unsupported pak footer magic");
    }

    pub fn read_with_format<R>(reader: &mut R, archive_len: u64, format: PakFormat) -> Result<Self>
    where
        R: Read + Seek,
    {
        let encrypted = Self::read_encrypted_footer(reader, archive_len)?;
        Self::try_parse_footer(encrypted, archive_len, format)?
            .with_context(|| format!("unsupported pak footer magic for {format:?}"))
    }

    fn read_encrypted_footer<R>(reader: &mut R, archive_len: u64) -> Result<[u8; FOOTER_SIZE]>
    where
        R: Read + Seek,
    {
        ensure!(
            archive_len >= FOOTER_SIZE as u64,
            "pak is too small to contain a footer"
        );

        let mut footer = [0_u8; FOOTER_SIZE];
        reader
            .seek(SeekFrom::End(-(FOOTER_SIZE as i64)))
            .context("failed to seek to pak footer")?;
        reader
            .read_exact(&mut footer)
            .context("failed to read pak footer")?;
        Ok(footer)
    }

    fn try_parse_footer(
        encrypted: [u8; FOOTER_SIZE],
        archive_len: u64,
        format: PakFormat,
    ) -> Result<Option<Self>> {
        let mut footer = encrypted;
        format.key().crypto().decrypt_in_place(&mut footer)?;

        let Some((file_count, extra_file_count)) = format.footer_counts(&footer) else {
            return Ok(None);
        };

        let record_count = file_count
            .checked_add(extra_file_count)
            .context("pak record count overflow")?;
        let record_bytes = record_count
            .checked_mul(RECORD_SIZE)
            .context("pak FAT byte count overflow")? as u64;
        let raw_fat_offset = archive_len
            .checked_sub(FOOTER_SIZE as u64)
            .and_then(|value| value.checked_sub(record_bytes))
            .context("pak FAT offset underflow")?;
        let fat_offset = raw_fat_offset - (raw_fat_offset % BLOCK_SIZE);

        Ok(Some(Self::new(
            format,
            file_count,
            extra_file_count,
            fat_offset,
            archive_len,
        )))
    }

    pub fn format(&self) -> PakFormat {
        self.format
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }

    pub fn extra_file_count(&self) -> usize {
        self.extra_file_count
    }

    pub fn record_count(&self) -> usize {
        self.file_count + self.extra_file_count
    }

    pub fn fat_offset(&self) -> u64 {
        self.fat_offset
    }

    pub fn archive_len(&self) -> u64 {
        self.archive_len
    }
}
