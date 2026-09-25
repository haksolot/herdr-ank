//! The herdr boundary (ADR-357c017baf9b): every action is an argv handed to
//! `$HERDR_BIN_PATH`, never a shell line, and the socket at
//! `$HERDR_SOCKET_PATH` is opened for `events.subscribe` only.
//!
//! herdr 0.9.1 offers no `--json` flag: its CLI always answers with one JSON
//! document, `{"id","result"}` on stdout and exit 0, or `{"id","error"}` on
//! stderr and a non-zero exit. Unknown fields are ignored throughout.

mod events;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::de::DeserializeOwned;
use serde::Deserialize;

pub use events::{Event, Subscription};

/// The `--source` under which the plugin reports pane metadata: the plugin id.
pub const SOURCE: &str = "ank";

#[derive(Debug)]
pub enum HerdrError {
    /// `HERDR_BIN_PATH` or `HERDR_SOCKET_PATH` is unset or empty.
    MissingEnv(&'static str),
    /// The binary could not be run, or the socket could not be used.
    Io { context: String, source: io::Error },
    /// herdr answered with an error body, from the CLI or the socket.
    Api { code: String, message: String },
    /// The CLI failed without an error body herdr's protocol describes.
    Exit { status: Option<i32>, stderr: String },
    /// herdr answered something this client cannot read.
    Decode(String),
}

impl fmt::Display for HerdrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HerdrError::MissingEnv(var) => write!(f, "{var} is not set"),
            HerdrError::Io { context, source } => write!(f, "{context}: {source}"),
            HerdrError::Api { code, message } => write!(f, "herdr {code}: {message}"),
            HerdrError::Exit { status, stderr } => match status {
                Some(code) => write!(f, "herdr exited with {code}: {}", stderr.trim()),
                None => write!(f, "herdr killed by a signal: {}", stderr.trim()),
            },
            HerdrError::Decode(what) => write!(f, "unreadable herdr answer: {what}"),
        }
    }
}

impl std::error::Error for HerdrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HerdrError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A pane as `pane list`, `agent list` and pane events describe it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Pane {
    pub pane_id: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub tab_id: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub agent_status: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    None,
    Done,
    Request,
}

impl Sound {
    fn as_arg(self) -> &'static str {
        match self {
            Sound::None => "none",
            Sound::Done => "done",
            Sound::Request => "request",
        }
    }
}

/// `herdr worktree create`.
#[derive(Debug, Clone)]
pub struct WorktreeCreate<'a> {
    pub workspace: &'a str,
    pub branch: &'a str,
    pub base: &'a str,
    pub path: &'a Path,
    pub label: Option<&'a str>,
    pub focus: bool,
}

/// `herdr tab create`.
#[derive(Debug, Clone)]
pub struct TabCreate<'a> {
    pub workspace: &'a str,
    pub cwd: &'a Path,
    pub env: &'a [(&'a str, &'a str)],
    pub label: Option<&'a str>,
    pub focus: bool,
}

#[derive(Debug, Clone)]
pub struct Client {
    bin: PathBuf,
    socket: PathBuf,
}

#[derive(Deserialize)]
struct Success<T> {
    result: T,
}

#[derive(Deserialize)]
struct Failure {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl From<ErrorBody> for HerdrError {
    fn from(body: ErrorBody) -> Self {
        HerdrError::Api {
            code: body.code,
            message: body.message,
        }
    }
}

#[derive(Deserialize)]
struct PaneList {
    panes: Vec<Pane>,
}

#[derive(Deserialize)]
struct AgentList {
    agents: Vec<Pane>,
}

#[derive(Deserialize)]
struct RootPane {
    root_pane: Pane,
}

#[derive(Deserialize)]
struct AgentResult {
    agent: Pane,
}

/// Any result: the fields are not read, only its presence is.
#[derive(Deserialize)]
struct Ignored {}

impl Client {
    pub fn new(bin: impl Into<PathBuf>, socket: impl Into<PathBuf>) -> Self {
        Client {
            bin: bin.into(),
            socket: socket.into(),
        }
    }

    /// The client herdr hands a plugin process through its environment.
    pub fn from_env() -> Result<Self, HerdrError> {
        let bin = env_path("HERDR_BIN_PATH")?;
        let socket = env_path("HERDR_SOCKET_PATH")?;
        Ok(Client::new(bin, socket))
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    pub fn pane_list(&self, workspace: Option<&str>) -> Result<Vec<Pane>, HerdrError> {
        let mut args = argv(["pane", "list"]);
        if let Some(workspace) = workspace {
            args.push("--workspace".into());
            args.push(workspace.into());
        }
        Ok(self.run::<PaneList>(args)?.panes)
    }

    pub fn agent_list(&self) -> Result<Vec<Pane>, HerdrError> {
        Ok(self.run::<AgentList>(argv(["agent", "list"]))?.agents)
    }

    /// Sets `tokens` and clears `clear` on `pane`, under the plugin's source.
    pub fn report_metadata(
        &self,
        pane: &str,
        tokens: &[(&str, &str)],
        clear: &[&str],
        ttl_ms: Option<u64>,
    ) -> Result<(), HerdrError> {
        let mut args = argv(["pane", "report-metadata", pane, "--source", SOURCE]);
        for (name, value) in tokens {
            args.push("--token".into());
            args.push(format!("{name}={value}").into());
        }
        for name in clear {
            args.push("--clear-token".into());
            args.push((*name).into());
        }
        if let Some(ttl) = ttl_ms {
            args.push("--ttl-ms".into());
            args.push(ttl.to_string().into());
        }
        self.run::<Ignored>(args).map(drop)
    }

    pub fn notification_show(
        &self,
        title: &str,
        body: Option<&str>,
        sound: Sound,
    ) -> Result<(), HerdrError> {
        let mut args = argv(["notification", "show", title]);
        if let Some(body) = body {
            args.push("--body".into());
            args.push(body.into());
        }
        args.push("--sound".into());
        args.push(sound.as_arg().into());
        self.run::<Ignored>(args).map(drop)
    }

    /// Creates the worktree and its tab; returns the tab's root pane.
    pub fn worktree_create(&self, req: &WorktreeCreate<'_>) -> Result<Pane, HerdrError> {
        let mut args = argv([
            "worktree",
            "create",
            "--workspace",
            req.workspace,
            "--branch",
            req.branch,
            "--base",
            req.base,
            "--path",
        ]);
        args.push(req.path.into());
        if let Some(label) = req.label {
            args.push("--label".into());
            args.push(label.into());
        }
        args.push(if req.focus { "--focus" } else { "--no-focus" }.into());
        Ok(self.run::<RootPane>(args)?.root_pane)
    }

    /// Creates a tab; returns its root pane.
    pub fn tab_create(&self, req: &TabCreate<'_>) -> Result<Pane, HerdrError> {
        let mut args = argv(["tab", "create", "--workspace", req.workspace, "--cwd"]);
        args.push(req.cwd.into());
        for (key, value) in req.env {
            args.push("--env".into());
            args.push(format!("{key}={value}").into());
        }
        if let Some(label) = req.label {
            args.push("--label".into());
            args.push(label.into());
        }
        args.push(if req.focus { "--focus" } else { "--no-focus" }.into());
        Ok(self.run::<RootPane>(args)?.root_pane)
    }

    /// Starts agent `name` of `kind` in `pane`, an interactive shell.
    pub fn agent_start(&self, name: &str, kind: &str, pane: &str) -> Result<Pane, HerdrError> {
        let args = argv(["agent", "start", name, "--kind", kind, "--pane", pane]);
        Ok(self.run::<AgentResult>(args)?.agent)
    }

    pub fn agent_prompt(&self, target: &str, text: &str) -> Result<(), HerdrError> {
        self.run::<Ignored>(argv(["agent", "prompt", target, text]))
            .map(drop)
    }

    /// Runs `herdr <args>` without a shell and decodes its `result`.
    fn run<T: DeserializeOwned>(&self, args: Vec<OsString>) -> Result<T, HerdrError> {
        let output = Command::new(&self.bin)
            .args(&args)
            .output()
            .map_err(|source| HerdrError::Io {
                context: format!("running {}", self.bin.display()),
                source,
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(match serde_json::from_str::<Failure>(&stderr) {
                Ok(failure) => failure.error.into(),
                Err(_) => HerdrError::Exit {
                    status: output.status.code(),
                    stderr,
                },
            });
        }
        serde_json::from_slice::<Success<T>>(&output.stdout)
            .map(|success| success.result)
            .map_err(|err| HerdrError::Decode(format!("{}: {err}", args_display(&args))))
    }
}

fn env_path(var: &'static str) -> Result<PathBuf, HerdrError> {
    match std::env::var_os(var) {
        Some(value) if !value.is_empty() => Ok(value.into()),
        _ => Err(HerdrError::MissingEnv(var)),
    }
}

fn argv<'a>(parts: impl IntoIterator<Item = &'a str>) -> Vec<OsString> {
    parts.into_iter().map(OsString::from).collect()
}

fn args_display(args: &[OsString]) -> String {
    args.iter()
        .take(2)
        .map(|a| a.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
