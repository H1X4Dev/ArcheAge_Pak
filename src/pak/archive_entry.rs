use anyhow::{Result, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    name: String,
    offset: u64,
    size: u64,
    size_duplicate: u64,
    padding_size: u32,
    md5: [u8; 16],
    dummy1: u32,
    create_time: i64,
    modify_time: i64,
    dummy2: u64,
}

impl ArchiveEntry {
    pub fn builder(name: impl Into<String>) -> ArchiveEntryBuilder {
        ArchiveEntryBuilder::new(name)
    }

    pub fn unused(offset: u64, slot_size: u64) -> Result<Self> {
        Self::builder("__unused__")
            .offset(offset)
            .size(slot_size)
            .size_duplicate(slot_size)
            .build()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn size_duplicate(&self) -> u64 {
        self.size_duplicate
    }

    pub fn padding_size(&self) -> u32 {
        self.padding_size
    }

    pub fn md5(&self) -> &[u8; 16] {
        &self.md5
    }

    pub fn dummy1(&self) -> u32 {
        self.dummy1
    }

    pub fn create_time(&self) -> i64 {
        self.create_time
    }

    pub fn modify_time(&self) -> i64 {
        self.modify_time
    }

    pub fn dummy2(&self) -> u64 {
        self.dummy2
    }

    pub fn slot_size(&self) -> u64 {
        self.size + u64::from(self.padding_size)
    }

    pub fn data_end(&self) -> u64 {
        self.offset + self.slot_size()
    }

    pub fn replace_in_place(&mut self, size: u64, md5: [u8; 16], modify_time: i64) {
        let old_end = self.data_end();
        self.size = size;
        self.size_duplicate = size;
        self.padding_size = (old_end - self.offset - size) as u32;
        self.md5 = md5;
        self.modify_time = modify_time;
        self.dummy1 = 0;
        self.dummy2 = 0;
    }

    pub fn replace_moved(
        &mut self,
        offset: u64,
        size: u64,
        padding_size: u32,
        md5: [u8; 16],
        modify_time: i64,
    ) {
        self.offset = offset;
        self.size = size;
        self.size_duplicate = size;
        self.padding_size = padding_size;
        self.md5 = md5;
        self.modify_time = modify_time;
        self.dummy1 = 0;
        self.dummy2 = 0;
    }
}

pub struct ArchiveEntryBuilder {
    name: String,
    offset: u64,
    size: u64,
    size_duplicate: Option<u64>,
    padding_size: u32,
    md5: [u8; 16],
    dummy1: u32,
    create_time: i64,
    modify_time: i64,
    dummy2: u64,
}

impl ArchiveEntryBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            offset: 0,
            size: 0,
            size_duplicate: None,
            padding_size: 0,
            md5: [0; 16],
            dummy1: 0,
            create_time: 0,
            modify_time: 0,
            dummy2: 0,
        }
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn size_duplicate(mut self, size_duplicate: u64) -> Self {
        self.size_duplicate = Some(size_duplicate);
        self
    }

    pub fn padding_size(mut self, padding_size: u32) -> Self {
        self.padding_size = padding_size;
        self
    }

    pub fn md5(mut self, md5: [u8; 16]) -> Self {
        self.md5 = md5;
        self
    }

    pub fn dummy1(mut self, dummy1: u32) -> Self {
        self.dummy1 = dummy1;
        self
    }

    pub fn create_time(mut self, create_time: i64) -> Self {
        self.create_time = create_time;
        self
    }

    pub fn modify_time(mut self, modify_time: i64) -> Self {
        self.modify_time = modify_time;
        self
    }

    pub fn dummy2(mut self, dummy2: u64) -> Self {
        self.dummy2 = dummy2;
        self
    }

    pub fn build(self) -> Result<ArchiveEntry> {
        ensure!(!self.name.is_empty(), "pak entry name cannot be empty");
        ensure!(
            self.name.len() <= 264,
            "pak entry name is longer than 264 bytes: {}",
            self.name
        );
        Ok(ArchiveEntry {
            name: self.name,
            offset: self.offset,
            size: self.size,
            size_duplicate: self.size_duplicate.unwrap_or(self.size),
            padding_size: self.padding_size,
            md5: self.md5,
            dummy1: self.dummy1,
            create_time: self.create_time,
            modify_time: self.modify_time,
            dummy2: self.dummy2,
        })
    }
}
