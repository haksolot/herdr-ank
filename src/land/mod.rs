//! Landing a finished task (ADR-ff4569057dc5): its branch `task/<short id>` is
//! rebased on the default branch in the task's worktree, then the default
//! branch fast-forwards to it in the integration tree.
//!
//! This is the only module that spawns git, the `git` on `PATH` with
//! `-C <path>`, and only for the operations the ADR lists. Every precondition
//! is read before the first write; a rebase that stops on a conflict is
//! aborted. [`land`] cleans nothing up; the action ([`action`]) removes the
//! branch after a landing through [`delete_branch`] and
//! [`delete_remote_branch`], and never pushes anything else.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::ank::Found;

pub mod action;

/// What a landing works on.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The tree on the default branch that the default branch moves in.
    pub integration: &'a Path,
    /// The task's own worktree, on `branch`.
    pub worktree: &'a Path,
    /// `task/<short id>`.
    pub branch: &'a str,
    pub default_branch: &'a str,
    /// The task as `ank find --json` reads it.
    pub task: &'a Found,
}

/// How a landing ended. Every variant but `Landed` is a refusal that leaves
/// the worktree, both branches and the trees in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landing {
    /// The default branch now points at `head`, the tip of the task branch.
    Landed { head: String },
    /// ank does not report the task done.
    NotDone { status: String },
    /// The task's worktree has changes `git status --porcelain` lists.
    DirtyWorktree,
    /// The integration tree has `head` checked out, not the default branch.
    IntegrationNotOnDefault { head: String },
    /// The integration tree has changes `git status --porcelain` lists.
    DirtyIntegration,
    /// The rebase stopped on a conflict; it was aborted, and the branch is
    /// back on the commit it had before.
    RebaseConflict,
    /// The default branch moved again after the one retry the landing allows.
    FastForwardRefused,
}

/// git itself failed, outside the refusals a landing expects.
#[derive(Debug)]
pub enum LandError {
    Spawn(io::Error),
    Git {
        args: String,
        stderr: String,
    },
    /// The worktree has `head` checked out, not the task branch.
    WorktreeNotOnBranch {
        head: String,
    },
}

impl fmt::Display for LandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LandError::Spawn(err) => write!(f, "cannot run git: {err}"),
            LandError::Git { args, stderr } => write!(f, "git {args} failed: {}", stderr.trim()),
            LandError::WorktreeNotOnBranch { head } => {
                write!(f, "the task's worktree is on {head}, not its branch")
            }
        }
    }
}

impl std::error::Error for LandError {}

/// Lands `request.branch` on `request.default_branch`.
pub fn land(request: &Request) -> Result<Landing, LandError> {
    let worktree = Git(request.worktree);
    let integration = Git(request.integration);

    if request.task.status != "done" {
        return Ok(Landing::NotDone {
            status: request.task.status.clone(),
        });
    }
    if worktree.dirty()? {
        return Ok(Landing::DirtyWorktree);
    }
    let head = worktree.head()?;
    if head != request.branch {
        return Err(LandError::WorktreeNotOnBranch { head });
    }
    let head = integration.head()?;
    if head != request.default_branch {
        return Ok(Landing::IntegrationNotOnDefault { head });
    }
    if integration.dirty()? {
        return Ok(Landing::DirtyIntegration);
    }

    for _ in 0..2 {
        if !worktree.rebase(request.default_branch)? {
            return Ok(Landing::RebaseConflict);
        }
        if integration.fast_forward(request.branch)? {
            let head = integration.read(&["rev-parse", request.default_branch])?;
            return Ok(Landing::Landed { head });
        }
    }
    Ok(Landing::FastForwardRefused)
}

/// `TASK-abcd00000000` -> `TASK-abcd`: the short id `work` names the branch
/// and the agent after.
pub fn short(id: &str) -> String {
    match id.split_once('-') {
        Some((kind, rest)) => format!("{kind}-{}", rest.get(..4).unwrap_or(rest)),
        None => id.to_owned(),
    }
}

/// `TASK-abcd00000000` -> `task/abcd`.
pub fn task_branch(id: &str) -> String {
    format!("task/{}", crate::work::short_id(&short(id)))
}

/// The tree the default branch moves in: the main checkout of the repository
/// `path` belongs to, `path` itself when it is that checkout.
pub fn integration_tree(path: &Path) -> Result<PathBuf, LandError> {
    let common = PathBuf::from(Git(path).read(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ])?);
    let main = common.parent().unwrap_or(&common).to_path_buf();
    // Keep the caller's spelling of the same directory: git writes its own.
    match (path.canonicalize(), main.canonicalize()) {
        (Ok(a), Ok(b)) if a == b => Ok(path.to_path_buf()),
        _ => Ok(main),
    }
}

/// Whether `branch` exists in `integration` and is not yet contained in
/// `default_branch`: what is left to land.
pub fn unlanded(integration: &Path, branch: &str, default_branch: &str) -> Result<bool, LandError> {
    let git = Git(integration);
    let local = format!("refs/heads/{branch}");
    if !git
        .output(&["rev-parse", "--verify", "--quiet", &local])?
        .status
        .success()
    {
        return Ok(false);
    }
    let args = ["merge-base", "--is-ancestor", &local, default_branch];
    let output = git.output(&args)?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(LandError::Git {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}

/// `branch -d <branch>` in the integration tree: git refuses a branch that is
/// not merged or that a worktree still has checked out.
pub fn delete_branch(integration: &Path, branch: &str) -> Result<(), LandError> {
    Git(integration).read(&["branch", "-d", branch]).map(drop)
}

/// `push origin --delete <branch>`, only when `ls-remote` finds the branch on
/// origin: false when it was not there.
pub fn delete_remote_branch(integration: &Path, branch: &str) -> Result<bool, LandError> {
    let git = Git(integration);
    let found = git.read(&[
        "ls-remote",
        "--heads",
        "origin",
        &format!("refs/heads/{branch}"),
    ])?;
    if found.is_empty() {
        return Ok(false);
    }
    git.read(&["push", "origin", "--delete", branch])?;
    Ok(true)
}

/// `git -C <path>`.
struct Git<'a>(&'a Path);

impl Git<'_> {
    fn output(&self, args: &[&str]) -> Result<Output, LandError> {
        Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(LandError::Spawn)
    }

    /// Runs a read and returns its trimmed stdout.
    fn read(&self, args: &[&str]) -> Result<String, LandError> {
        let output = self.output(args)?;
        if !output.status.success() {
            return Err(LandError::Git {
                args: args.join(" "),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn dirty(&self) -> Result<bool, LandError> {
        Ok(!self.read(&["status", "--porcelain"])?.is_empty())
    }

    /// The checked-out branch, `HEAD` when detached.
    fn head(&self) -> Result<String, LandError> {
        self.read(&["rev-parse", "--abbrev-ref", "HEAD"])
    }

    /// `rebase <onto>`: false when it stopped and was aborted.
    fn rebase(&self, onto: &str) -> Result<bool, LandError> {
        let output = self.output(&["rebase", onto])?;
        if output.status.success() {
            return Ok(true);
        }
        if !self.rebasing()? {
            return Err(LandError::Git {
                args: format!("rebase {onto}"),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        self.read(&["rebase", "--abort"])?;
        Ok(false)
    }

    /// Whether a rebase is in progress in this tree.
    fn rebasing(&self) -> Result<bool, LandError> {
        for dir in ["rebase-merge", "rebase-apply"] {
            let path = PathBuf::from(self.read(&[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                dir,
            ])?);
            if path.exists() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `merge --ff-only <branch>`: false when git refused it.
    fn fast_forward(&self, branch: &str) -> Result<bool, LandError> {
        Ok(self
            .output(&["merge", "--ff-only", branch])?
            .status
            .success())
    }
}
