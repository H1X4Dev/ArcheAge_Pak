use super::BLOCK_SIZE;

pub struct BlockAlignment;

impl BlockAlignment {
    pub fn align_offset(value: u64) -> u64 {
        let remainder = value % BLOCK_SIZE;
        if remainder == 0 {
            value
        } else {
            value + (BLOCK_SIZE - remainder)
        }
    }

    pub fn padding_for_size(size: u64) -> u64 {
        (BLOCK_SIZE - (size % BLOCK_SIZE)) % BLOCK_SIZE
    }
}
