use std::io::{Seek, SeekFrom, Write};

use anyhow::{Context, Result, ensure};

use super::{ArchiveEntry, BLOCK_SIZE, FOOTER_SIZE, FOOTER_USED_SIZE, PakCrypto, RecordCodec};

pub struct ArchiveWriter {
    codec: RecordCodec,
    crypto: PakCrypto,
}

impl ArchiveWriter {
    pub fn xl_games() -> Self {
        let crypto = PakCrypto::xl_games();
        Self::new(crypto)
    }

    pub fn new(crypto: PakCrypto) -> Self {
        Self {
            codec: RecordCodec::new(crypto.clone()),
            crypto,
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

        for entry in entries.iter().chain(extras.iter()) {
            let record = self.codec.encode(entry)?;
            writer
                .write_all(&record)
                .context("failed to write pak FAT record")?;
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
        footer_plain[0..4].copy_from_slice(b"WIBO");
        footer_plain[8..12].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        footer_plain[12..16].copy_from_slice(&(extras.len() as u32).to_le_bytes());
        self.crypto.encrypt_in_place(&mut footer_plain)?;

        let mut footer = [0_u8; FOOTER_SIZE];
        footer[..FOOTER_USED_SIZE].copy_from_slice(&footer_plain[..FOOTER_USED_SIZE]);
        writer
            .write_all(&footer)
            .context("failed to write pak footer")?;

        Ok(fat_offset + records_len + padding_len + FOOTER_SIZE as u64)
    }
}
