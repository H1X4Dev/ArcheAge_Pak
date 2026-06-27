use super::PakCrypto;

const XL_GAMES_KEY: [u8; 16] = [
    0x32, 0x1f, 0x2a, 0xee, 0xaa, 0x58, 0x4a, 0xb4, 0x9a, 0x6c, 0x9e, 0x09, 0xd5, 0x9e, 0x9c, 0x6f,
];

const ARCHERAGE_KEY: [u8; 16] = [
    0x6f, 0xf6, 0x6a, 0xb5, 0x11, 0xc0, 0x42, 0x69, 0xea, 0x96, 0x97, 0x9d, 0x51, 0x82, 0x98, 0x14,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PakKey {
    #[default]
    XlGames,
    Archerage,
}

impl PakKey {
    pub fn crypto(self) -> PakCrypto {
        PakCrypto::new(self.bytes())
    }

    fn bytes(self) -> [u8; 16] {
        match self {
            Self::XlGames => XL_GAMES_KEY,
            Self::Archerage => ARCHERAGE_KEY,
        }
    }
}
