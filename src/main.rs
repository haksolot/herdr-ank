use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use herdr_ank::{ank, config, herdr, pick, work};

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
    /// The popup `work` opens: choose a claimable task.
    Pick,
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Command::Daemon => "daemon",
            Command::Sync => "sync",
            Command::Work => "work",
            Command::Tui => "tui",
            Command::Pick => "pick",
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Tui = cli.command {
        return herdr_ank::tui::run();
    }
    let result = match cli.command {
        Command::Work => run_work(),
        Command::Pick => run_pick(),
        other => Err(format!("herdr-ank {}: not implemented yet", other.name())),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn env_path(var: &str) -> Result<PathBuf, String> {
    match std::env::var_os(var) {
        Some(value) if !value.is_empty() => Ok(value.into()),
        _ => Err(format!(
            "herdr-ank: {var} is not set; herdr sets it for a plugin"
        )),
    }
}

fn run_work() -> Result<(), String> {
    // ank must compute the bare <user>@<host> the agent's identity is built on,
    // not the identity of whatever pane herdr was started from.
    std::env::remove_var("ANK_AGENT");
    let config =
        config::Config::load(&env_path("HERDR_PLUGIN_CONFIG_DIR")?).map_err(|e| e.to_string())?;
    let work = work::Work {
        herdr: herdr::Client::from_env().map_err(|e| e.to_string())?,
        ank_program: "ank".into(),
        context_json: std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default(),
        state_dir: env_path("HERDR_PLUGIN_STATE_DIR")?,
        worktrees_root: env_path("HOME")?.join(".herdr/worktrees"),
        config,
        poll: Duration::from_millis(250),
        pick_timeout: Duration::from_secs(600),
        claim_timeout: Duration::from_secs(180),
    };
    let outcome = work::run(&work).map_err(|e| format!("herdr-ank work: {e}"))?;
    println!("{outcome:?}");
    Ok(())
}

fn run_pick() -> Result<(), String> {
    let state_dir = env_path("HERDR_PLUGIN_STATE_DIR")?;
    let chosen: Result<Option<String>, String> = (|| {
        let repo = env_path(work::REPO_ENV)?;
        let context = ank::Client::new(&repo)
            .context(None)
            .map_err(|e| e.to_string())?;
        match pick::run_terminal(pick::claimable(context.tasks)).map_err(|e| e.to_string())? {
            pick::Step::Selected(id) => Ok(Some(id)),
            _ => Ok(None),
        }
    })();
    // Whatever happened, `work` is waiting: a failure answers as a cancel.
    let written =
        pick::write_selection(&state_dir, chosen.as_ref().ok().and_then(|c| c.as_deref()));
    chosen?;
    written.map_err(|e| format!("herdr-ank pick: {e}"))
}
