use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod link;
pub mod os;
pub mod registry;
pub mod style;

pub fn run(cli: cli::Cli) -> ExitCode {
    init_tracing(cli.verbose);

    match commands::dispatch(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn init_tracing(verbose: bool) {
    let default = if verbose { "dotup=debug" } else { "dotup=warn" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init();
}
