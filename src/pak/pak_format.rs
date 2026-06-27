use super::{FOOTER_SIZE, PakKey};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PakFormat {
    #[default]
    XlGames,
    Archerage,
}

impl PakFormat {
    pub fn key(self) -> PakKey {
        match self {
            Self::XlGames => PakKey::XlGames,
            Self::Archerage => PakKey::Archerage,
        }
    }

    pub(crate) fn known() -> &'static [Self] {
        &[Self::XlGames, Self::Archerage]
    }

    pub(crate) fn footer_counts(self, footer: &[u8; FOOTER_SIZE]) -> Option<(usize, usize)> {
        match self {
            Self::XlGames => {
                if &footer[0..4] != b"WIBO" {
                    return None;
                }
                Some((
                    Self::read_u32(footer, 8) as usize,
                    Self::read_u32(footer, 12) as usize,
                ))
            }
            Self::Archerage => {
                if &footer[8..12] != b"IDEJ" {
                    return None;
                }
                Some((
                    Self::read_u32(footer, 12) as usize,
                    Self::read_u32(footer, 0) as usize,
                ))
            }
        }
    }

    pub(crate) fn write_footer(self, footer: &mut [u8; FOOTER_SIZE], files: usize, extras: usize) {
        match self {
            Self::XlGames => {
                footer[0..4].copy_from_slice(b"WIBO");
                footer[8..12].copy_from_slice(&(files as u32).to_le_bytes());
                footer[12..16].copy_from_slice(&(extras as u32).to_le_bytes());
            }
            Self::Archerage => {
                footer[0..4].copy_from_slice(&(extras as u32).to_le_bytes());
                footer[8..12].copy_from_slice(b"IDEJ");
                footer[12..16].copy_from_slice(&(files as u32).to_le_bytes());
            }
        }
    }

    pub(crate) fn default_dummy1(self, value: u32) -> u32 {
        match self {
            Self::XlGames => value,
            Self::Archerage if value == 0 => 0x8000_0000,
            Self::Archerage => value,
        }
    }

    pub(crate) fn stores_extras_first(self) -> bool {
        matches!(self, Self::Archerage)
    }

    fn read_u32(bytes: &[u8; FOOTER_SIZE], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("slice length"))
    }
}
