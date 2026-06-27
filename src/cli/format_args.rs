use clap::{Args, ValueEnum};

use crate::pak::PakFormat;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum FormatArg {
    #[default]
    #[value(name = "xlgames")]
    XlGames,
    Archerage,
}

impl From<FormatArg> for PakFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::XlGames => Self::XlGames,
            FormatArg::Archerage => Self::Archerage,
        }
    }
}

#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Pak output format.
    #[arg(long, value_enum, default_value_t = FormatArg::XlGames)]
    pub format: FormatArg,
}

impl FormatArgs {
    pub fn pak_format(&self) -> PakFormat {
        self.format.into()
    }
}
