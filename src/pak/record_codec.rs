use anyhow::{Context, Result, ensure};

use super::{ArchiveEntry, NAME_SIZE, PakCrypto, RECORD_SIZE};

pub struct RecordCodec {
    crypto: PakCrypto,
}

impl RecordCodec {
    pub fn new(crypto: PakCrypto) -> Self {
        Self { crypto }
    }

    pub fn decode(&self, encrypted: &[u8]) -> Result<ArchiveEntry> {
        ensure!(
            encrypted.len() == RECORD_SIZE,
            "invalid pak record size: {}",
            encrypted.len()
        );
        let mut record = [0_u8; RECORD_SIZE];
        record.copy_from_slice(encrypted);
        self.crypto.decrypt_in_place(&mut record)?;

        let name_len = record[..NAME_SIZE]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NAME_SIZE);
        let name = String::from_utf8_lossy(&record[..name_len]).into_owned();
        let offset = u64::from_le_bytes(record[264..272].try_into().expect("slice length"));
        let size = u64::from_le_bytes(record[272..280].try_into().expect("slice length"));
        let size_duplicate = u64::from_le_bytes(record[280..288].try_into().expect("slice length"));
        let padding_size = u32::from_le_bytes(record[288..292].try_into().expect("slice length"));
        let mut md5 = [0_u8; 16];
        md5.copy_from_slice(&record[292..308]);
        let dummy1 = u32::from_le_bytes(record[308..312].try_into().expect("slice length"));
        let create_time = i64::from_le_bytes(record[312..320].try_into().expect("slice length"));
        let modify_time = i64::from_le_bytes(record[320..328].try_into().expect("slice length"));
        let dummy2 = u64::from_le_bytes(record[328..336].try_into().expect("slice length"));

        ArchiveEntry::builder(name)
            .offset(offset)
            .size(size)
            .size_duplicate(size_duplicate)
            .padding_size(padding_size)
            .md5(md5)
            .dummy1(dummy1)
            .create_time(create_time)
            .modify_time(modify_time)
            .dummy2(dummy2)
            .build()
    }

    pub fn encode(&self, entry: &ArchiveEntry) -> Result<[u8; RECORD_SIZE]> {
        let mut record = [0_u8; RECORD_SIZE];
        let name_bytes = entry.name().as_bytes();
        ensure!(
            name_bytes.len() <= NAME_SIZE,
            "pak entry name is longer than {NAME_SIZE} bytes: {}",
            entry.name()
        );
        record[..name_bytes.len()].copy_from_slice(name_bytes);
        record[264..272].copy_from_slice(&entry.offset().to_le_bytes());
        record[272..280].copy_from_slice(&entry.size().to_le_bytes());
        record[280..288].copy_from_slice(&entry.size_duplicate().to_le_bytes());
        record[288..292].copy_from_slice(&entry.padding_size().to_le_bytes());
        record[292..308].copy_from_slice(entry.md5());
        record[308..312].copy_from_slice(&entry.dummy1().to_le_bytes());
        record[312..320].copy_from_slice(&entry.create_time().to_le_bytes());
        record[320..328].copy_from_slice(&entry.modify_time().to_le_bytes());
        record[328..336].copy_from_slice(&entry.dummy2().to_le_bytes());
        self.crypto
            .encrypt_in_place(&mut record)
            .context("failed to encrypt pak record")?;
        Ok(record)
    }
}
