use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = dotup::cli::Cli::parse();
    dotup::run(cli)
}
