use std::process::ExitCode;

use archeage_pak::{cli::Cli, commands::CommandRunner};
use clap::Parser;

fn main() -> ExitCode {
    match CommandRunner::new().run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
