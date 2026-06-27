use std::io::{Seek, SeekFrom, Write};

use anyhow::{Context, Result, ensure};

use super::{ArchiveEntry, BLOCK_SIZE, FOOTER_SIZE, FOOTER_USED_SIZE, PakFormat, RecordCodec};

pub struct ArchiveWriter {
    codec: RecordCodec,
    format: PakFormat,
}

impl ArchiveWriter {
    pub fn xl_games() -> Self {
        Self::for_format(PakFormat::XlGames)
    }

    pub fn for_format(format: PakFormat) -> Self {
        Self {
            codec: RecordCodec::for_format(format),
            format,
        }
    }

    pub fn write_to<W>(
        &self,
        writer: &mut W,
        fat_offset: u64,
        entries: &[ArchiveEntry],
        extras: &[ArchiveEntry],
    ) -> Result<u64>
    where
        W: Seek + Write,
    {
        ensure!(
            fat_offset.is_multiple_of(BLOCK_SIZE),
            "FAT offset must be 512-byte aligned"
        );

        writer
            .seek(SeekFrom::Start(fat_offset))
            .context("failed to seek to output FAT offset")?;

        if self.format.stores_extras_first() {
            for entry in extras.iter().chain(entries.iter()) {
                self.write_record(writer, entry)?;
            }
        } else {
            for entry in entries.iter().chain(extras.iter()) {
                self.write_record(writer, entry)?;
            }
        }

        let records_len = ((entries.len() + extras.len()) * super::RECORD_SIZE) as u64;
        let padding_len = (BLOCK_SIZE - (records_len % BLOCK_SIZE)) % BLOCK_SIZE;
        if padding_len > 0 {
            let padding = vec![0_u8; padding_len as usize];
            writer
                .write_all(&padding)
                .context("failed to write pak FAT padding")?;
        }

        let mut footer_plain = [0_u8; FOOTER_SIZE];
        self.format
            .write_footer(&mut footer_plain, entries.len(), extras.len());
        self.format
            .key()
            .crypto()
            .encrypt_in_place(&mut footer_plain)?;

        let mut footer = [0_u8; FOOTER_SIZE];
        footer[..FOOTER_USED_SIZE].copy_from_slice(&footer_plain[..FOOTER_USED_SIZE]);
        writer
            .write_all(&footer)
            .context("failed to write pak footer")?;

        Ok(fat_offset + records_len + padding_len + FOOTER_SIZE as u64)
    }

    fn write_record<W>(&self, writer: &mut W, entry: &ArchiveEntry) -> Result<()>
    where
        W: Write,
    {
        let record = self.codec.encode(entry)?;
        writer
            .write_all(&record)
            .context("failed to write pak FAT record")
    }
}
