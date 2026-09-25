//! The `work` action: one agent, one tree, one identity (ADR-fa8b6a103597),
//! without the user typing a command. Pick a claimable task in a popup, cut
//! its worktree, open a tab carrying `ANK_AGENT`, start the agent there and
//! have it claim the task.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::ank::{self, AnkError};
use crate::config::Config;
use crate::herdr::{self, HerdrError, Sound, TabCreate, WorktreeCreate};
use crate::pick;

/// The pane entry the manifest declares for the picker.
pub const PICK_PANE: &str = "pick";
/// How `work` tells the picker which corpus to list.
pub const REPO_ENV: &str = "HERDR_ANK_REPO";

/// Everything `work` reaches outside itself.
#[derive(Debug, Clone)]
pub struct Work {
    pub herdr: herdr::Client,
    /// The `ank` binary; `ank` on `PATH` outside tests.
    pub ank_program: PathBuf,
    /// `$HERDR_PLUGIN_CONTEXT_JSON`.
    pub context_json: String,
    /// `$HERDR_PLUGIN_STATE_DIR`, where the picker leaves its choice.
    pub state_dir: PathBuf,
    /// Worktrees go to `<worktrees_root>/<corpus directory name>/<agent name>`.
    pub worktrees_root: PathBuf,
    pub config: Config,
    /// How often the selection and the claim are looked for.
    pub poll: Duration,
    pub pick_timeout: Duration,
    /// How long the agent has to claim before the user is told it did not.
    pub claim_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The picker was closed without a choice.
    Cancelled,
    /// The agent holds the task.
    Claimed {
        id: String,
        agent: String,
        worktree: PathBuf,
    },
    /// Somebody else holds the task, or it was finished elsewhere: what exit 4
    /// of `ank claim` means. The user was notified; the worktree stays.
    Unavailable {
        id: String,
        holder: Option<String>,
        worktree: PathBuf,
    },
    /// No claim was seen before `claim_timeout`. The user was notified.
    NotObserved {
        id: String,
        agent: String,
        worktree: PathBuf,
    },
}

#[derive(Debug)]
pub enum WorkError {
    /// `$HERDR_PLUGIN_CONTEXT_JSON` is unreadable.
    Context(serde_json::Error),
    /// The invocation names no workspace.
    NoWorkspace,
    /// The invocation names no directory to look for a corpus from.
    NoCwd,
    /// No directory from `searched` up holds `.ank/`.
    NoCorpus {
        searched: PathBuf,
    },
    /// The picker never answered.
    PickTimeout,
    /// The picker chose an id the corpus does not list as claimable.
    UnknownTask(String),
    State(io::Error),
    Ank(AnkError),
    Herdr(HerdrError),
}

impl fmt::Display for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkError::Context(e) => write!(f, "unreadable HERDR_PLUGIN_CONTEXT_JSON: {e}"),
            WorkError::NoWorkspace => {
                write!(f, "this action runs in a workspace, and none was given")
            }
            WorkError::NoCwd => write!(f, "the workspace has no directory to look for a corpus in"),
            WorkError::NoCorpus { searched } => write!(
                f,
                "no .ank/ in {} or any directory above it\n  -> ank init",
                searched.display()
            ),
            WorkError::PickTimeout => write!(f, "the picker was left open without a choice"),
            WorkError::UnknownTask(id) => write!(f, "{id} is not a claimable task of this corpus"),
            WorkError::State(e) => write!(f, "plugin state directory: {e}"),
            WorkError::Ank(e) => write!(f, "{e}"),
            WorkError::Herdr(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for WorkError {}

impl From<AnkError> for WorkError {
    fn from(e: AnkError) -> Self {
        WorkError::Ank(e)
    }
}

impl From<HerdrError> for WorkError {
    fn from(e: HerdrError) -> Self {
        WorkError::Herdr(e)
    }
}

/// The part of herdr's `PluginInvocationContext` this action reads.
#[derive(Debug, Deserialize)]
struct Invocation {
    workspace_id: Option<String>,
    workspace_cwd: Option<PathBuf>,
    focused_pane_cwd: Option<PathBuf>,
}

/// The first directory from `start` up that holds `.ank/`.
pub fn find_corpus(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(".ank").is_dir())
        .map(Path::to_path_buf)
}

/// `TASK-c81a` -> `c81a`: the short id without its kind.
pub fn short_id(short: &str) -> String {
    short
        .split_once('-')
        .map_or(short, |(_, rest)| rest)
        .to_lowercase()
}

pub fn run(work: &Work) -> Result<Outcome, WorkError> {
    let invocation: Invocation =
        serde_json::from_str(&work.context_json).map_err(WorkError::Context)?;
    let workspace = invocation.workspace_id.ok_or(WorkError::NoWorkspace)?;
    let cwd = invocation
        .workspace_cwd
        .or(invocation.focused_pane_cwd)
        .ok_or(WorkError::NoCwd)?;
    let corpus = find_corpus(&cwd).ok_or(WorkError::NoCorpus { searched: cwd })?;
    let ank = ank::Client::with_program(&work.ank_program, &corpus);

    let Some(id) = pick_task(work, &workspace, &corpus)? else {
        return Ok(Outcome::Cancelled);
    };
    let task = pick::claimable(ank.context(None)?.tasks)
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| WorkError::UnknownTask(id.clone()))?;

    let status = ank.status()?;
    let base = status.default_branch.unwrap_or_else(|| {
        eprintln!(
            "herdr-ank work: the corpus names no default branch, the worktree starts from HEAD"
        );
        "HEAD".to_owned()
    });
    // The fallback identity ank computes, whatever an agent suffix it carries.
    let user_host = status
        .identity
        .value
        .split('/')
        .next()
        .unwrap_or_default()
        .to_owned();

    let short = short_id(&task.short);
    let agent = format!("ank-{short}");
    let branch = format!("task/{short}");
    let corpus_name = corpus.file_name().map_or("corpus".into(), |n| n.to_owned());
    let worktree = work.worktrees_root.join(corpus_name).join(&agent);
    let identity = format!("{user_host}/{agent}");

    work.herdr.worktree_create(&WorktreeCreate {
        workspace: &workspace,
        branch: &branch,
        base: &base,
        path: &worktree,
        label: Some(&agent),
        focus: false,
    })?;
    let tab = work.herdr.tab_create(&TabCreate {
        workspace: &workspace,
        cwd: &worktree,
        env: &[("ANK_AGENT", &identity)],
        label: Some(&agent),
        focus: true,
    })?;
    work.herdr
        .agent_start(&agent, &work.config.agent.kind, &tab.pane_id)?;
    work.herdr
        .agent_prompt(&agent, &format!("ank claim {}", task.id))?;

    // The agent runs the claim, so its exit code never reaches this process:
    // what the claim left in the corpus is read instead.
    let deadline = Instant::now() + work.claim_timeout;
    loop {
        let found = ank.find(&[&task.id])?;
        if let Some(row) = found.results.iter().find(|r| r.id == task.id) {
            match row.state.strip_prefix("claimed:") {
                Some(holder) if holder == identity => {
                    return Ok(Outcome::Claimed {
                        id: task.id,
                        agent,
                        worktree,
                    })
                }
                Some(holder) => {
                    let body = format!(
                        "{} is held by {holder}; the worktree {} stays",
                        task.short,
                        worktree.display()
                    );
                    work.herdr.notification_show(
                        "ank: task unavailable",
                        Some(&body),
                        Sound::Request,
                    )?;
                    return Ok(Outcome::Unavailable {
                        id: task.id,
                        holder: Some(holder.to_owned()),
                        worktree,
                    });
                }
                None if row.status != "open" => {
                    let body = format!(
                        "{} is {} elsewhere; the worktree {} stays",
                        task.short,
                        row.status,
                        worktree.display()
                    );
                    work.herdr.notification_show(
                        "ank: task unavailable",
                        Some(&body),
                        Sound::Request,
                    )?;
                    return Ok(Outcome::Unavailable {
                        id: task.id,
                        holder: None,
                        worktree,
                    });
                }
                None => {}
            }
        }
        if Instant::now() >= deadline {
            let body = format!("{agent} has not claimed {} yet; see its pane", task.short);
            work.herdr
                .notification_show("ank: claim not seen", Some(&body), Sound::Request)?;
            return Ok(Outcome::NotObserved {
                id: task.id,
                agent,
                worktree,
            });
        }
        thread::sleep(work.poll);
    }
}

/// Opens the picker and waits for its choice: `None` for a cancel.
fn pick_task(work: &Work, workspace: &str, corpus: &Path) -> Result<Option<String>, WorkError> {
    let selection = pick::selection_path(&work.state_dir);
    match fs::remove_file(&selection) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(WorkError::State(e)),
        _ => {}
    }
    let corpus = corpus.to_string_lossy();
    work.herdr
        .plugin_pane_open(PICK_PANE, Some(workspace), &[(REPO_ENV, &corpus)])?;

    let deadline = Instant::now() + work.pick_timeout;
    loop {
        match fs::read_to_string(&selection) {
            Ok(id) => {
                let id = id.trim();
                return Ok((!id.is_empty()).then(|| id.to_owned()));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(WorkError::State(e)),
        }
        if Instant::now() >= deadline {
            return Err(WorkError::PickTimeout);
        }
        thread::sleep(work.poll);
    }
}
