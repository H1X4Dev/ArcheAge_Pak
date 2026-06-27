use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    commands::{
        create::CreateCommand, extract_all::ExtractAllCommand, extract_file::ExtractFileCommand,
        list::ListCommand, replace::ReplaceCommand,
    },
};

pub struct CommandRunner;

impl CommandRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, cli: Cli) -> Result<()> {
        match cli.command() {
            Command::List(args) => ListCommand::new(args).execute(),
            Command::ExtractAll(args) => ExtractAllCommand::new(args).execute(),
            Command::ExtractFile(args) => ExtractFileCommand::new(args).execute(),
            Command::Create(args) => CreateCommand::new(args).execute(),
            Command::Replace(args) => ReplaceCommand::new(args).execute(),
        }
    }
}

impl Default for CommandRunner {
    fn default() -> Self {
        Self::new()
    }
}
