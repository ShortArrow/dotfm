use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config not found at {path}. run `dotfm init` first")]
    ConfigMissing { path: PathBuf },

    #[error("config already exists at {path}. use --force to overwrite")]
    ConfigExists { path: PathBuf },

    #[error("dotfiles root {path} does not contain dotfm.toml")]
    RegistryMissing { path: PathBuf },

    #[error("could not determine dotfiles root. pass --dotfiles <path> or run `dotfm init` first")]
    DotfilesRootUnknown,

    #[error("unknown tool `{name}`. available: {available}")]
    UnknownTool { name: String, available: String },

    #[error("tool `{name}` is not enabled on this machine")]
    NotEnabled { name: String },

    #[error("tool `{name}` has no configuration for the current OS")]
    UnsupportedOs { name: String },

    #[error(
        "destination {path} exists and is not a symlink managed by dotfm; use --force to back it up and replace"
    )]
    DestinationOccupied { path: PathBuf },
}
