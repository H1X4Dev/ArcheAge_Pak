use aes::Aes128;
use anyhow::{Result, anyhow};
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::NoPadding};

use super::DEFAULT_KEY;

type Aes128CbcDecryptor = cbc::Decryptor<Aes128>;
type Aes128CbcEncryptor = cbc::Encryptor<Aes128>;

#[derive(Clone)]
pub struct PakCrypto {
    key: [u8; 16],
}

impl PakCrypto {
    pub fn xl_games() -> Self {
        Self::new(DEFAULT_KEY)
    }

    pub fn new(key: [u8; 16]) -> Self {
        Self { key }
    }

    pub fn decrypt_in_place(&self, bytes: &mut [u8]) -> Result<()> {
        let iv = [0_u8; 16];
        Aes128CbcDecryptor::new(&self.key.into(), &iv.into())
            .decrypt_padded_mut::<NoPadding>(bytes)
            .map_err(|_| anyhow!("failed to decrypt pak metadata"))?;
        Ok(())
    }

    pub fn encrypt_in_place(&self, bytes: &mut [u8]) -> Result<()> {
        let iv = [0_u8; 16];
        Aes128CbcEncryptor::new(&self.key.into(), &iv.into())
            .encrypt_padded_mut::<NoPadding>(bytes, bytes.len())
            .map_err(|_| anyhow!("failed to encrypt pak metadata"))?;
        Ok(())
    }
}
