//! Plugin configuration, read from `config.toml` in the directory
//! `herdr plugin config-dir ank` prints. A missing file or key takes its
//! default; an unknown key is ignored; a key of the wrong type is an error
//! that names it.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

pub const FILE_NAME: &str = "config.toml";

/// ADR-6fb76f3a1197 wants a sync at least every 30 s, and the sidebar tokens'
/// 90 s ttl (SPEC-dbe3cf972f71) is three times that maximum.
pub const MAX_POLL_SECONDS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub agent: AgentConfig,
    pub sync: SyncConfig,
    pub notify: NotifyConfig,
}

/// The herdr agent the plugin starts for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    pub kind: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConfig {
    pub poll_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyConfig {
    pub done: bool,
    pub expiring_minutes: u64,
    pub review: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            agent: AgentConfig {
                kind: "claude".to_owned(),
                args: Vec::new(),
            },
            sync: SyncConfig { poll_seconds: 30 },
            notify: NotifyConfig {
                done: true,
                expiring_minutes: 10,
                review: true,
            },
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        message: String,
    },
    /// A well-typed value outside the range the key allows.
    Range {
        key: String,
        min: u64,
        max: u64,
    },
    /// `key` is dotted from the root, e.g. `sync.poll_seconds`.
    Type {
        key: String,
        expected: &'static str,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Parse { path, message } => write!(f, "{}: {message}", path.display()),
            Error::Range { key, min, max } => {
                write!(f, "config key `{key}`: must be between {min} and {max}")
            }
            Error::Type { key, expected } => write!(f, "config key `{key}`: expected {expected}"),
        }
    }
}

impl std::error::Error for Error {}

impl Config {
    /// Reads `config.toml` in `dir`, or the defaults when it does not exist.
    pub fn load(dir: &Path) -> Result<Config, Error> {
        let path = dir.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let table: Table = text.parse().map_err(|e: toml::de::Error| Error::Parse {
                    path,
                    message: e.message().to_owned(),
                })?;
                Config::from_table(&table)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(source) => Err(Error::Read { path, source }),
        }
    }

    fn from_table(root: &Table) -> Result<Config, Error> {
        let mut config = Config::default();
        if let Some(agent) = table(root, "agent")? {
            if let Some(kind) = agent.get("kind") {
                config.agent.kind = string(kind, "agent.kind")?;
            }
            if let Some(args) = agent.get("args") {
                config.agent.args = strings(args, "agent.args")?;
            }
        }
        if let Some(sync) = table(root, "sync")? {
            if let Some(v) = sync.get("poll_seconds") {
                config.sync.poll_seconds = bounded(v, "sync.poll_seconds", 1, MAX_POLL_SECONDS)?;
            }
        }
        if let Some(notify) = table(root, "notify")? {
            if let Some(v) = notify.get("done") {
                config.notify.done = boolean(v, "notify.done")?;
            }
            if let Some(v) = notify.get("expiring_minutes") {
                config.notify.expiring_minutes = unsigned(v, "notify.expiring_minutes")?;
            }
            if let Some(v) = notify.get("review") {
                config.notify.review = boolean(v, "notify.review")?;
            }
        }
        Ok(config)
    }
}

fn mistyped(key: &str, expected: &'static str) -> Error {
    Error::Type {
        key: key.to_owned(),
        expected,
    }
}

fn table<'a>(root: &'a Table, key: &str) -> Result<Option<&'a Table>, Error> {
    match root.get(key) {
        None => Ok(None),
        Some(Value::Table(t)) => Ok(Some(t)),
        Some(_) => Err(mistyped(key, "a table")),
    }
}

fn string(v: &Value, key: &str) -> Result<String, Error> {
    v.as_str()
        .map(str::to_owned)
        .ok_or_else(|| mistyped(key, "a string"))
}

fn strings(v: &Value, key: &str) -> Result<Vec<String>, Error> {
    v.as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|i| i.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| mistyped(key, "an array of strings"))
}

fn unsigned(v: &Value, key: &str) -> Result<u64, Error> {
    v.as_integer()
        .and_then(|i| u64::try_from(i).ok())
        .ok_or_else(|| mistyped(key, "a non-negative integer"))
}

fn bounded(v: &Value, key: &str, min: u64, max: u64) -> Result<u64, Error> {
    let n = unsigned(v, key)?;
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(Error::Range {
            key: key.to_owned(),
            min,
            max,
        })
    }
}

fn boolean(v: &Value, key: &str) -> Result<bool, Error> {
    v.as_bool().ok_or_else(|| mistyped(key, "a boolean"))
}
