use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use herdr_ank::{ank, config, herdr, land, pick, work};

/// herdr plugin for ank: every manifest entry is a subcommand (ADR-599b6f424271).
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
    /// Open the `tui` overlay over the invoking workspace ([[actions]] open).
    Open,
    /// Pick a finished task, land it, then clean up as `[land]` says.
    Land,
    /// The popup `work` and `land` open: choose a task.
    Pick,
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Command::Daemon => "daemon",
            Command::Sync => "sync",
            Command::Work => "work",
            Command::Tui => "tui",
            Command::Open => "open",
            Command::Land => "land",
            Command::Pick => "pick",
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Tui = cli.command {
        return herdr_ank::tui::run();
    }
    if let Command::Open = cli.command {
        return herdr_ank::tui::open();
    }
    let result = match cli.command {
        Command::Work => run_work(),
        Command::Land => run_land(),
        Command::Pick => run_pick(),
        Command::Daemon => herdr_ank::daemon::run(),
        Command::Sync => herdr_ank::daemon::sync(),
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

/// The variable naming the user's home, under which worktrees go.
#[cfg(unix)]
const HOME: &str = "HOME";
#[cfg(windows)]
const HOME: &str = "USERPROFILE";

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
        ank_program: ank::program(),
        context_json: std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default(),
        state_dir: env_path("HERDR_PLUGIN_STATE_DIR")?,
        worktrees_root: env_path(HOME)?.join(".herdr").join("worktrees"),
        config,
        poll: Duration::from_millis(250),
        pick_timeout: Duration::from_secs(600),
        claim_timeout: Duration::from_secs(180),
    };
    let outcome = work::run(&work).map_err(|e| format!("herdr-ank work: {e}"))?;
    println!("{outcome:?}");
    Ok(())
}

fn run_land() -> Result<(), String> {
    let herdr = herdr::Client::from_env().map_err(|e| e.to_string())?;
    let result = (|| {
        let config = config::Config::load(&env_path("HERDR_PLUGIN_CONFIG_DIR")?)
            .map_err(|e| e.to_string())?;
        let action = land::action::Land {
            herdr: herdr.clone(),
            ank_program: ank::program(),
            context_json: std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default(),
            state_dir: env_path("HERDR_PLUGIN_STATE_DIR")?,
            config: config.land,
            poll: Duration::from_millis(250),
            pick_timeout: Duration::from_secs(600),
        };
        land::action::run(&action).map_err(|e| e.to_string())
    })();
    match result {
        Ok(outcome) => {
            println!("{outcome:?}");
            Ok(())
        }
        Err(message) => {
            // An action's stderr reaches nobody: the user is told in herdr.
            let _ =
                herdr.notification_show("ank: land failed", Some(&message), herdr::Sound::Request);
            Err(format!("herdr-ank land: {message}"))
        }
    }
}

fn run_pick() -> Result<(), String> {
    let state_dir = env_path("HERDR_PLUGIN_STATE_DIR")?;
    let mode = pick::Mode::from_env_value(std::env::var(pick::MODE_ENV).ok().as_deref());
    let chosen: Result<Option<String>, String> = (|| {
        let repo = env_path(work::REPO_ENV)?;
        let ank = ank::Client::new(&repo);
        let tasks = match mode {
            pick::Mode::Work => {
                pick::claimable(ank.context(None).map_err(|e| e.to_string())?.tasks)
            }
            pick::Mode::Land => land::action::landable_tasks(&ank).map_err(|e| e.to_string())?,
        };
        match pick::run_terminal(tasks, mode).map_err(|e| e.to_string())? {
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
