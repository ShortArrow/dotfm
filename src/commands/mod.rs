use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{Cli, Cmd};
use crate::style::Icons;

pub mod add;
pub mod apply;
pub mod doctor;
pub mod init;
pub mod list;
pub mod remove;
pub mod status;
pub mod util;

pub fn dispatch(cli: Cli) -> Result<ExitCode> {
    let icons = Icons::resolve(cli.icons);
    match cli.command {
        Cmd::Init { force } => init::run(cli.dotfiles.as_deref(), force),
        Cmd::Add {
            ref tools,
            apply: run_apply,
        } => add::run(
            cli.dotfiles.as_deref(),
            tools,
            run_apply,
            cli.dry_run,
            false,
            icons,
        ),
        Cmd::Remove { ref tools } => {
            remove::run(cli.dotfiles.as_deref(), tools, cli.dry_run, icons)
        }
        Cmd::Apply { ref tools, force } => {
            apply::run(cli.dotfiles.as_deref(), tools, force, cli.dry_run, icons)
        }
        Cmd::Status => status::run(cli.dotfiles.as_deref(), icons),
        Cmd::List => list::run(cli.dotfiles.as_deref(), icons),
        Cmd::Doctor {
            ref tools,
            all,
            no_generic,
        } => doctor::run(cli.dotfiles.as_deref(), tools, all, no_generic, icons),
    }
}
