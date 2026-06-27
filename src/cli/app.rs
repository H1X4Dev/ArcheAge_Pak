use clap::Parser;

use super::Command;

#[derive(Debug, Parser)]
#[command(author, version, about = "ArcheAge game_pak extractor and writer")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn command(self) -> Command {
        self.command
    }
}
