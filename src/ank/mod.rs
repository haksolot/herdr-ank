//! The boundary with ank (ADR-3cd19cd6acb9): everything the plugin knows of a
//! corpus comes through `ank <verb> --json --repo <repo>`, and nothing else in
//! the crate spawns `ank`.
//!
//! The plugin reads as `<user>@<host>/herdr-ank`: an identity that never holds
//! a claim, so `context` stays in orientation mode even while the bare
//! `<user>@<host>` holds one, and no herdr agent `work` starts can carry it,
//! their names beginning with `ank-` (ADR-fa8b6a103597).

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::de::DeserializeOwned;

mod types;

pub use types::*;

/// The only document contract this client reads.
pub const CONTRACT: u64 = 1;

/// The agent name of the plugin's own identity.
pub const PLUGIN_AGENT: &str = "herdr-ank";

const ANK_AGENT: &str = "ANK_AGENT";

/// Runs ank over one repository.
#[derive(Debug, Clone)]
pub struct Client {
    program: PathBuf,
    repo: PathBuf,
    /// `<user>@<host>/herdr-ank`, asked of ank once.
    identity: OnceLock<String>,
}

impl Client {
    /// A client running the `ank` [`program`] finds.
    pub fn new(repo: &Path) -> Self {
        Self::with_program(program(), repo)
    }

    /// A client running the given `ank` binary.
    pub fn with_program(program: impl Into<PathBuf>, repo: &Path) -> Self {
        Self {
            program: program.into(),
            repo: repo.to_path_buf(),
            identity: OnceLock::new(),
        }
    }

    /// `ank <verb> <args> --json --repo <repo>`, run as the plugin's identity.
    pub fn command<S: AsRef<OsStr>>(&self, verb: &str, args: &[S]) -> Result<Command, AnkError> {
        let mut command = self.bare(verb, args);
        command.env(ANK_AGENT, self.identity()?);
        Ok(command)
    }

    /// The plugin's identity on this corpus: the fallback `<user>@<host>` ank
    /// computes itself (asked with `ANK_AGENT` removed), suffixed `/herdr-ank`.
    fn identity(&self) -> Result<&str, AnkError> {
        if let Some(identity) = self.identity.get() {
            return Ok(identity);
        }
        let mut probe = self.bare("status", &[] as &[&str]);
        probe.env_remove(ANK_AGENT);
        let status: Status = read(probe)?;
        let user_host = status.identity.value.split('/').next().unwrap_or_default();
        Ok(self
            .identity
            .get_or_init(|| format!("{user_host}/{PLUGIN_AGENT}")))
    }

    fn bare<S: AsRef<OsStr>>(&self, verb: &str, args: &[S]) -> Command {
        let mut command = Command::new(&self.program);
        command
            .arg(verb)
            .args(args)
            .arg("--json")
            .arg("--repo")
            .arg(&self.repo);
        command
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
        read(self.command(verb, args)?)
    }
}

/// Runs `command` and reads its stdout as a document of this contract.
fn read<T: DeserializeOwned>(mut command: Command) -> Result<T, AnkError> {
    let output = command.output().map_err(AnkError::Spawn)?;
    check(&output)?;
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(AnkError::Json)?;
    let contract = value.get("contract").and_then(serde_json::Value::as_u64);
    if contract != Some(CONTRACT) {
        return Err(AnkError::Contract { found: contract });
    }
    serde_json::from_value(value).map_err(AnkError::Json)
}

/// The exit code, routed before anything is parsed: a refusal leaves stdout empty.
fn check(output: &std::process::Output) -> Result<(), AnkError> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Err(match output.status.code() {
        Some(code @ 1..=9) => AnkError::Exit {
            code: code as u8,
            kind: ExitKind::from_code(code as u8),
            stderr,
        },
        code => AnkError::Abnormal { code, stderr },
    })
}

/// `events.jsonl` beside the `watch.yml` that `ank watch --where` names; the
/// file itself may not exist yet. `watch` addresses no corpus and refuses
/// `--repo`, so it reads nothing as anyone: it runs without the plugin
/// identity, which only a corpus can tell.
pub fn events_jsonl(program: &Path) -> Result<PathBuf, AnkError> {
    let output = Command::new(program)
        .args(["watch", "--where"])
        .output()
        .map_err(AnkError::Spawn)?;
    check(&output)?;
    let watch_yml = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    Ok(watch_yml
        .parent()
        .unwrap_or(Path::new(""))
        .join("events.jsonl"))
}

/// The `ank` every call of the plugin runs, looked up once. herdr often runs
/// as a service whose `PATH` lacks the user's own bin directories, so `PATH`
/// comes first and then where ank is usually installed: next to herdr,
/// `<home>/.local/bin`, `<home>/.cargo/bin`, and on Unix `/opt/homebrew/bin`
/// and `/usr/local/bin`. Found nowhere, it is plain `ank`, and running it
/// fails as it always did.
pub fn program() -> PathBuf {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(locate).clone()
}

fn locate() -> PathBuf {
    let name = format!("ank{}", std::env::consts::EXE_SUFFIX);
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if let Some(herdr) = std::env::var_os("HERDR_BIN_PATH") {
        if let Some(dir) = Path::new(&herdr).parent() {
            if !dir.as_os_str().is_empty() {
                dirs.push(dir.to_path_buf());
            }
        }
    }
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|home| !home.is_empty()));
    if let Some(home) = home.map(PathBuf::from) {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".cargo").join("bin"));
    }
    if cfg!(unix) {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    dirs.into_iter()
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from("ank"))
}

/// `ank tui` for the human at the pane: run with the corpus as its cwd, and
/// not `--repo`, which ank 0.8.0's TUI passes to its child calls before the
/// verb, where the CLI rejects it (haksolot/ank#495). The environment is left
/// as the user has it, `ANK_AGENT` included: what they claim from the TUI is
/// theirs.
pub fn tui_command(repo: &Path) -> Command {
    let mut command = Command::new(program());
    command.arg("tui").current_dir(repo);
    command
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
