use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = dotfm::cli::Cli::parse();
    dotfm::run(cli)
}
