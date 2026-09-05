use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "herdr-hop",
    version,
    about = "Hop to any visible Herdr pane by label"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Action entrypoint: label the visible panes and open the picker popup.
    Jump,

    /// Popup entrypoint: paint labels, read one keystroke, and jump. Runs inside the popup pane.
    Pick,
}

pub fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Jump => crate::action::run(),
        Command::Pick => crate::picker::run(),
    }
}
