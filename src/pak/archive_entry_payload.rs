use crate::io::CopyOutcome;

pub(crate) struct ArchiveEntryPayload {
    offset: u64,
    size: u64,
    padding_size: u32,
    md5: [u8; 16],
    create_time: i64,
    modify_time: i64,
}

impl ArchiveEntryPayload {
    pub(crate) fn from_copy_outcome(
        offset: u64,
        padding_size: u32,
        outcome: &CopyOutcome,
        create_time: i64,
        modify_time: i64,
    ) -> Self {
        Self {
            offset,
            size: outcome.bytes(),
            padding_size,
            md5: outcome.md5(),
            create_time,
            modify_time,
        }
    }

    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }

    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    pub(crate) fn padding_size(&self) -> u32 {
        self.padding_size
    }

    pub(crate) fn md5(&self) -> [u8; 16] {
        self.md5
    }

    pub(crate) fn create_time(&self) -> i64 {
        self.create_time
    }

    pub(crate) fn modify_time(&self) -> i64 {
        self.modify_time
    }
}
