//! The boundary with ank (ADR-3cd19cd6acb9): everything the plugin knows of a
//! corpus comes through `ank <verb> --json --repo <repo>`, and nothing else in
//! the crate spawns `ank`.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::de::DeserializeOwned;

mod types;

pub use types::*;

/// The only document contract this client reads.
pub const CONTRACT: u64 = 1;

/// Runs ank over one repository.
#[derive(Debug, Clone)]
pub struct Client {
    program: PathBuf,
    repo: PathBuf,
}

impl Client {
    /// A client running the `ank` found on `PATH`.
    pub fn new(repo: &Path) -> Self {
        Self::with_program("ank", repo)
    }

    /// A client running the given `ank` binary.
    pub fn with_program(program: impl Into<PathBuf>, repo: &Path) -> Self {
        Self {
            program: program.into(),
            repo: repo.to_path_buf(),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// `ank status`.
    pub fn status(&self) -> Result<Status, AnkError> {
        self.run("status", &[] as &[&str])
    }

    /// `ank find <args>`: the query and its flags, e.g. `["--status", "open", "--free"]`.
    pub fn find(&self, args: &[&str]) -> Result<Find, AnkError> {
        self.run("find", args)
    }

    /// `ank show <id>`.
    pub fn show(&self, id: &str) -> Result<Show, AnkError> {
        self.run("show", &[id])
    }

    /// `ank context [<path>]`.
    pub fn context(&self, path: Option<&Path>) -> Result<Context, AnkError> {
        match path {
            Some(path) => self.run("context", &[path]),
            None => self.run("context", &[] as &[&str]),
        }
    }

    fn run<T: DeserializeOwned, S: AsRef<OsStr>>(
        &self,
        verb: &str,
        args: &[S],
    ) -> Result<T, AnkError> {
        let output = Command::new(&self.program)
            .arg(verb)
            .args(args)
            .arg("--json")
            .arg("--repo")
            .arg(&self.repo)
            .output()
            .map_err(AnkError::Spawn)?;

        // The code is routed before anything is parsed: a refusal leaves stdout empty.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(match output.status.code() {
                Some(code @ 1..=9) => AnkError::Exit {
                    code: code as u8,
                    kind: ExitKind::from_code(code as u8),
                    stderr,
                },
                code => AnkError::Abnormal { code, stderr },
            });
        }

        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(AnkError::Json)?;
        let contract = value.get("contract").and_then(serde_json::Value::as_u64);
        if contract != Some(CONTRACT) {
            return Err(AnkError::Contract { found: contract });
        }
        serde_json::from_value(value).map_err(AnkError::Json)
    }
}

/// What an exit code means, as ank's exit-code reference names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// 1: a call the parser refuses, a file the tool cannot make sense of.
    Generic,
    /// 2: no such entity, or a prefix matching more than one.
    NotFound,
    /// 3: the entity moved under the caller; read again.
    Conflict,
    /// 4: held by another agent or finished elsewhere; take something else.
    Unavailable,
    /// 5: a proof is missing, malformed, or of the wrong type.
    Proof,
    /// 6: the state forbids the act.
    Transition,
    /// 7: the act is legal and something it depends on is absent.
    Prerequisite,
    /// 8: `check` or `review` found a fault.
    Findings,
    /// 9: the environment, not the work.
    Environment,
}

impl ExitKind {
    /// The kind of a code in 1..=9; anything else is reported as `Generic`.
    pub fn from_code(code: u8) -> Self {
        match code {
            2 => Self::NotFound,
            3 => Self::Conflict,
            4 => Self::Unavailable,
            5 => Self::Proof,
            6 => Self::Transition,
            7 => Self::Prerequisite,
            8 => Self::Findings,
            9 => Self::Environment,
            _ => Self::Generic,
        }
    }
}

#[derive(Debug)]
pub enum AnkError {
    /// ank could not be started.
    Spawn(io::Error),
    /// ank refused: its exit code, what that code means, and its stderr.
    Exit {
        code: u8,
        kind: ExitKind,
        stderr: String,
    },
    /// ank exited outside 0..=9, or was killed by a signal (`code` is `None`).
    Abnormal { code: Option<i32>, stderr: String },
    /// The document's `contract` is not the one this client reads.
    Contract { found: Option<u64> },
    /// stdout is not the document the contract describes.
    Json(serde_json::Error),
}

impl AnkError {
    pub fn kind(&self) -> Option<ExitKind> {
        match self {
            Self::Exit { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Exit 9: the environment needs repair, the work did not fail.
    pub fn is_environment(&self) -> bool {
        self.kind() == Some(ExitKind::Environment)
    }
}

impl fmt::Display for AnkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "cannot run ank: {e}"),
            Self::Exit { code, stderr, .. } => write!(f, "ank exited {code}: {}", stderr.trim()),
            Self::Abnormal {
                code: Some(code),
                stderr,
            } => write!(f, "ank exited {code}: {}", stderr.trim()),
            Self::Abnormal { code: None, stderr } => {
                write!(f, "ank was killed by a signal: {}", stderr.trim())
            }
            Self::Contract { found: Some(n) } => {
                write!(f, "ank document contract {n}, this client reads {CONTRACT}")
            }
            Self::Contract { found: None } => {
                write!(
                    f,
                    "ank document has no contract, this client reads {CONTRACT}"
                )
            }
            Self::Json(e) => write!(f, "unreadable ank document: {e}"),
        }
    }
}

impl std::error::Error for AnkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
