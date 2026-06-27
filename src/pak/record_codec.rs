use anyhow::{Context, Result, ensure};

use super::{ArchiveEntry, NAME_SIZE, PakCrypto, PakFormat, RECORD_SIZE};

pub struct RecordCodec {
    format: PakFormat,
    crypto: PakCrypto,
}

impl RecordCodec {
    pub fn for_format(format: PakFormat) -> Self {
        Self::new(format, format.key().crypto())
    }

    pub fn new(format: PakFormat, crypto: PakCrypto) -> Self {
        Self { format, crypto }
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

        match self.format {
            PakFormat::XlGames => Self::decode_xl_games(&record),
            PakFormat::Archerage => Self::decode_archerage(&record),
        }
    }

    pub fn encode(&self, entry: &ArchiveEntry) -> Result<[u8; RECORD_SIZE]> {
        let mut record = match self.format {
            PakFormat::XlGames => Self::encode_xl_games(entry)?,
            PakFormat::Archerage => Self::encode_archerage(entry)?,
        };
        self.crypto
            .encrypt_in_place(&mut record)
            .context("failed to encrypt pak record")?;
        Ok(record)
    }

    fn decode_xl_games(record: &[u8; RECORD_SIZE]) -> Result<ArchiveEntry> {
        let name_len = record[0..NAME_SIZE]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NAME_SIZE);
        let name = String::from_utf8_lossy(&record[0..name_len]).into_owned();
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

    fn decode_archerage(record: &[u8; RECORD_SIZE]) -> Result<ArchiveEntry> {
        let name_offset = 32;
        let name_end = name_offset + NAME_SIZE;
        let name_len = record[name_offset..name_end]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NAME_SIZE);
        let name =
            String::from_utf8_lossy(&record[name_offset..name_offset + name_len]).into_owned();
        let padding_size = u32::from_le_bytes(record[0..4].try_into().expect("slice length"));
        let mut md5 = [0_u8; 16];
        md5.copy_from_slice(&record[4..20]);
        let dummy1 = u32::from_le_bytes(record[20..24].try_into().expect("slice length"));
        let size = u64::from_le_bytes(record[24..32].try_into().expect("slice length"));
        let size_duplicate = u64::from_le_bytes(record[296..304].try_into().expect("slice length"));
        let offset = u64::from_le_bytes(record[304..312].try_into().expect("slice length"));
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

    fn encode_xl_games(entry: &ArchiveEntry) -> Result<[u8; RECORD_SIZE]> {
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
        Ok(record)
    }

    fn encode_archerage(entry: &ArchiveEntry) -> Result<[u8; RECORD_SIZE]> {
        let mut record = [0_u8; RECORD_SIZE];
        let name_bytes = entry.name().as_bytes();
        ensure!(
            name_bytes.len() <= NAME_SIZE,
            "pak entry name is longer than {NAME_SIZE} bytes: {}",
            entry.name()
        );
        record[0..4].copy_from_slice(&entry.padding_size().to_le_bytes());
        record[4..20].copy_from_slice(entry.md5());
        record[20..24].copy_from_slice(
            &PakFormat::Archerage
                .default_dummy1(entry.dummy1())
                .to_le_bytes(),
        );
        record[24..32].copy_from_slice(&entry.size().to_le_bytes());
        record[32..32 + name_bytes.len()].copy_from_slice(name_bytes);
        record[296..304].copy_from_slice(&entry.size_duplicate().to_le_bytes());
        record[304..312].copy_from_slice(&entry.offset().to_le_bytes());
        record[312..320].copy_from_slice(&entry.create_time().to_le_bytes());
        record[320..328].copy_from_slice(&entry.modify_time().to_le_bytes());
        record[328..336].copy_from_slice(&entry.dummy2().to_le_bytes());
        Ok(record)
    }
}
