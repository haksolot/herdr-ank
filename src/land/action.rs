//! The `land` action: pick a finished task in a popup, land its branch
//! (ADR-ff4569057dc5), then clean up after it as `[land]` says, in order:
//! the tabs sitting in its worktree, the worktree, the local branch, the
//! branch on origin. A refusal is notified with the command that lifts it
//! and cleans nothing; a cleanup step that fails is notified and the next
//! ones still run, since the landing itself stands.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::ank::{self, AnkError, ContextTask};
use crate::config::LandConfig;
use crate::herdr::{self, HerdrError, Sound, Worktree};
use crate::land::{self as landing, LandError, Landing, Request};
use crate::pick::{self, ChooseError};
use crate::work;

/// Everything `land` reaches outside itself.
#[derive(Debug, Clone)]
pub struct Land {
    pub herdr: herdr::Client,
    /// The `ank` binary; `ank` on `PATH` outside tests.
    pub ank_program: PathBuf,
    /// `$HERDR_PLUGIN_CONTEXT_JSON`.
    pub context_json: String,
    /// `$HERDR_PLUGIN_STATE_DIR`, where the picker leaves its choice.
    pub state_dir: PathBuf,
    pub config: LandConfig,
    /// How often the selection is looked for.
    pub poll: Duration,
    pub pick_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The picker was closed without a choice.
    Cancelled,
    /// Nothing moved; the user was told `cause` and what lifts it.
    Refused { id: String, cause: String },
    /// The default branch is at `head`. Each cleanup step that failed is in
    /// `failures`, and was notified.
    Landed {
        id: String,
        head: String,
        failures: Vec<String>,
    },
}

#[derive(Debug)]
pub enum ActionError {
    /// `$HERDR_PLUGIN_CONTEXT_JSON` is unreadable.
    Context(serde_json::Error),
    NoWorkspace,
    NoCwd,
    NoCorpus {
        searched: PathBuf,
    },
    NoDefaultBranch,
    /// The picker chose an id the corpus does not list.
    UnknownTask(String),
    Pick(ChooseError),
    Ank(AnkError),
    Herdr(HerdrError),
    Git(LandError),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::Context(e) => write!(f, "unreadable HERDR_PLUGIN_CONTEXT_JSON: {e}"),
            ActionError::NoWorkspace => {
                write!(f, "this action runs in a workspace, and none was given")
            }
            ActionError::NoCwd => {
                write!(f, "the workspace has no directory to look for a corpus in")
            }
            ActionError::NoCorpus { searched } => write!(
                f,
                "no .ank/ in {} or any directory above it\n  -> ank init",
                searched.display()
            ),
            ActionError::NoDefaultBranch => write!(
                f,
                "ank names no default branch for this corpus, so there is nowhere to land\n  -> ank status"
            ),
            ActionError::UnknownTask(id) => write!(f, "{id} is not a task of this corpus"),
            ActionError::Pick(e) => write!(f, "{e}"),
            ActionError::Ank(e) => write!(f, "{e}"),
            ActionError::Herdr(e) => write!(f, "{e}"),
            ActionError::Git(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<AnkError> for ActionError {
    fn from(e: AnkError) -> Self {
        ActionError::Ank(e)
    }
}

impl From<HerdrError> for ActionError {
    fn from(e: HerdrError) -> Self {
        ActionError::Herdr(e)
    }
}

impl From<LandError> for ActionError {
    fn from(e: LandError) -> Self {
        ActionError::Git(e)
    }
}

impl From<ChooseError> for ActionError {
    fn from(e: ChooseError) -> Self {
        ActionError::Pick(e)
    }
}

/// The part of herdr's `PluginInvocationContext` this action reads.
#[derive(Debug, Deserialize)]
struct Invocation {
    workspace_id: Option<String>,
    workspace_cwd: Option<PathBuf>,
    focused_pane_cwd: Option<PathBuf>,
}

/// What the land picker lists: the done tasks of the corpus at `ank`'s repo
/// whose branch `task/<short id>` exists and is not yet in the default branch.
pub fn landable_tasks(ank: &ank::Client) -> Result<Vec<ContextTask>, ActionError> {
    let default_branch = ank
        .status()?
        .default_branch
        .ok_or(ActionError::NoDefaultBranch)?;
    let integration = landing::integration_tree(ank.repo())?;
    let found = ank.find(&["--type", "task", "--status", "done"])?;
    let mut tasks = Vec::new();
    for row in found.results {
        if row.status != "done"
            || !landing::unlanded(
                &integration,
                &landing::task_branch(&row.id),
                &default_branch,
            )?
        {
            continue;
        }
        tasks.push(ContextTask {
            short: landing::short(&row.id),
            id: row.id,
            title: row.title,
            status: row.status,
            ready: true,
            unblocks: 0,
            state: row.state,
        });
    }
    Ok(tasks)
}

pub fn run(land: &Land) -> Result<Outcome, ActionError> {
    let invocation: Invocation =
        serde_json::from_str(&land.context_json).map_err(ActionError::Context)?;
    let workspace = invocation.workspace_id.ok_or(ActionError::NoWorkspace)?;
    let cwd = invocation
        .workspace_cwd
        .or(invocation.focused_pane_cwd)
        .ok_or(ActionError::NoCwd)?;
    let corpus = work::find_corpus(&cwd).ok_or(ActionError::NoCorpus { searched: cwd })?;
    let integration = landing::integration_tree(&corpus)?;
    let ank = ank::Client::with_program(&land.ank_program, &integration);

    let repo = integration.to_string_lossy();
    let Some(id) = pick::choose(
        &land.herdr,
        &land.state_dir,
        &[(work::REPO_ENV, &repo), (pick::MODE_ENV, "land")],
        land.poll,
        land.pick_timeout,
    )?
    else {
        return Ok(Outcome::Cancelled);
    };
    let task = ank
        .find(&[&id])?
        .results
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| ActionError::UnknownTask(id.clone()))?;
    let default_branch = ank
        .status()?
        .default_branch
        .ok_or(ActionError::NoDefaultBranch)?;
    let short = landing::short(&id);
    let branch = landing::task_branch(&id);

    let worktrees = land.herdr.worktree_list(&workspace)?;
    let Some(worktree) = worktrees
        .into_iter()
        .find(|w| w.branch.as_deref() == Some(branch.as_str()))
    else {
        let cause = format!("no worktree has {branch} checked out");
        let remedy = format!("herdr worktree open --branch {branch}");
        return refuse(land, id, &short, cause, &remedy);
    };

    let request = Request {
        integration: &integration,
        worktree: &worktree.path,
        branch: &branch,
        default_branch: &default_branch,
        task: &task,
    };
    let head = match landing::land(&request) {
        Ok(Landing::Landed { head }) => head,
        Ok(refusal) => {
            let (cause, remedy) = explain_refusal(&refusal, &request, &short);
            return refuse(land, id, &short, cause, &remedy);
        }
        Err(err) => {
            let (cause, remedy) = explain_error(&err, &request);
            return refuse(land, id, &short, cause, &remedy);
        }
    };

    let failures = clean_up(land, &integration, &worktree, &branch, &short);
    notify(
        land,
        &format!("{short} landée sur {default_branch}"),
        None,
        Sound::Done,
    );
    Ok(Outcome::Landed { id, head, failures })
}

/// Tells the user why nothing moved and what lifts it.
fn refuse(
    land: &Land,
    id: String,
    short: &str,
    cause: String,
    remedy: &str,
) -> Result<Outcome, ActionError> {
    let body = format!("{cause}\n  -> {remedy}");
    land.herdr.notification_show(
        &format!("ank: {short} not landed"),
        Some(&body),
        Sound::Request,
    )?;
    Ok(Outcome::Refused { id, cause })
}

fn explain_refusal(refusal: &Landing, request: &Request, short: &str) -> (String, String) {
    let worktree = request.worktree.display();
    let integration = request.integration.display();
    let default = request.default_branch;
    match refusal {
        Landing::NotDone { status } => (
            format!("{short} is {status}, not done"),
            format!("ank done, in {worktree}"),
        ),
        Landing::DirtyWorktree => (
            format!("the worktree {worktree} has uncommitted changes"),
            format!("git -C {worktree} status, then commit them"),
        ),
        Landing::IntegrationNotOnDefault { head } => (
            format!("the integration tree {integration} is on {head}, not {default}"),
            format!("git -C {integration} switch {default}"),
        ),
        Landing::DirtyIntegration => (
            format!("the integration tree {integration} has uncommitted changes"),
            format!("git -C {integration} status"),
        ),
        Landing::RebaseConflict => (
            format!(
                "rebasing {} on {default} stops on a conflict; the rebase was aborted",
                request.branch
            ),
            format!("git -C {worktree} rebase {default}, resolve, then Land a task again"),
        ),
        Landing::FastForwardRefused => (
            format!("{default} moved again while {} was rebased", request.branch),
            "Land a task again".to_owned(),
        ),
        Landing::Landed { .. } => unreachable!("a landing is no refusal"),
    }
}

fn explain_error(err: &LandError, request: &Request) -> (String, String) {
    let worktree = request.worktree.display();
    let remedy = match err {
        LandError::WorktreeNotOnBranch { .. } => {
            format!("git -C {worktree} switch {}", request.branch)
        }
        LandError::Spawn(_) => "install git and put it on PATH".to_owned(),
        LandError::Git { .. } => format!("git -C {worktree} status"),
    };
    (err.to_string(), remedy)
}

/// The cleanup steps `[land]` enables, in order; returns what failed.
fn clean_up(
    land: &Land,
    integration: &Path,
    worktree: &Worktree,
    branch: &str,
    short: &str,
) -> Vec<String> {
    let config = &land.config;
    let mut failures = Vec::new();
    let mut fail = |what: String| {
        notify(
            land,
            &format!("ank: {short} landed, cleanup failed"),
            Some(&what),
            Sound::Request,
        );
        failures.push(what);
    };

    if config.close_tab {
        match tabs_in(land, worktree) {
            Ok(tabs) => {
                for tab in tabs {
                    if let Err(e) = land.herdr.tab_close(&tab) {
                        fail(format!("herdr tab close {tab}: {e}"));
                    }
                }
            }
            Err(e) => fail(format!("herdr pane list: {e}")),
        }
    }
    if config.remove_worktree {
        let path = worktree.path.display();
        match &worktree.open_workspace_id {
            Some(workspace) => {
                if let Err(e) = land.herdr.worktree_remove(workspace) {
                    fail(format!("herdr worktree remove {path}: {e}"));
                }
            }
            None => fail(format!(
                "the worktree {path} is open in no workspace\n  -> herdr worktree open --path {path}, then herdr worktree remove"
            )),
        }
    }
    if config.delete_branch {
        if let Err(e) = landing::delete_branch(integration, branch) {
            fail(format!("git branch -d {branch}: {e}"));
        }
    }
    if config.delete_remote_branch {
        if let Err(e) = landing::delete_remote_branch(integration, branch) {
            fail(format!("git push origin --delete {branch}: {e}"));
        }
    }
    failures
}

/// The tabs, in every workspace, where a pane has its cwd under the worktree.
/// When the worktree is to be removed, the tabs of its own workspace are
/// left to `worktree remove`: closing the last of them closes that workspace,
/// and herdr would then have no workspace to remove the worktree through.
fn tabs_in(land: &Land, worktree: &Worktree) -> Result<Vec<String>, HerdrError> {
    let spared = worktree
        .open_workspace_id
        .as_deref()
        .filter(|_| land.config.remove_worktree);
    let mut tabs: Vec<String> = Vec::new();
    for pane in land.herdr.pane_list(None)? {
        let inside = pane
            .cwd
            .as_deref()
            .is_some_and(|cwd| cwd.starts_with(&worktree.path));
        if inside && spared != Some(pane.workspace_id.as_str()) && !tabs.contains(&pane.tab_id) {
            tabs.push(pane.tab_id);
        }
    }
    Ok(tabs)
}

/// A notification after the landing: failing to show it moves nothing back.
fn notify(land: &Land, title: &str, body: Option<&str>, sound: Sound) {
    if let Err(e) = land.herdr.notification_show(title, body, sound) {
        eprintln!("herdr-ank land: notification {title:?} not shown: {e}");
    }
}
