use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// herdr plugin for ank: every manifest entry is a subcommand (ADR-aca6aaeb3a5f).
#[derive(Parser)]
#[command(name = "herdr-ank", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Keep herdr in sync with the ank corpus ([[startup]] entry).
    Daemon,
    /// Resynchronise once and exit.
    Sync,
    /// Pick a task, open its worktree and agent, claim it.
    Work,
    /// Open `ank tui` over the current workspace's corpus.
    Tui,
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Command::Daemon => "daemon",
            Command::Sync => "sync",
            Command::Work => "work",
            Command::Tui => "tui",
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    eprintln!("herdr-ank {}: not implemented yet", cli.command.name());
    ExitCode::from(1)
}
