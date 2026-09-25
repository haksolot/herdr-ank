//! `events.subscribe` over `$HERDR_SOCKET_PATH`, read as NDJSON: one request
//! line out, an acknowledgement line back, then one event per line.

use std::io::{BufRead, BufReader, Lines, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Client, ErrorBody, HerdrError, Pane};

/// One entry of `events.subscribe`'s `subscriptions`. Some kinds need a pane:
/// herdr 0.9.1 refuses `pane.agent_status_changed` without a `pane_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subscription {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pane_id: Option<String>,
}

impl Subscription {
    /// A kind as herdr spells it in a subscription, dotted: `pane.updated`.
    pub fn new(kind: &str) -> Self {
        Subscription {
            kind: kind.into(),
            pane_id: None,
        }
    }

    pub fn for_pane(kind: &str, pane: &str) -> Self {
        Subscription {
            kind: kind.into(),
            pane_id: Some(pane.into()),
        }
    }
}

/// The events this plugin reads. Any other kind is skipped by the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    PaneCreated(Pane),
    PaneUpdated(Pane),
    PaneClosed {
        pane_id: String,
        workspace_id: String,
    },
    PaneAgentStatusChanged {
        pane_id: String,
        workspace_id: String,
        agent: Option<String>,
        agent_status: String,
    },
    PaneAgentDetected {
        pane_id: String,
        workspace_id: String,
        agent: Option<String>,
    },
    WorktreeCreated {
        workspace_id: String,
        path: PathBuf,
        branch: Option<String>,
    },
    WorktreeOpened {
        workspace_id: String,
        path: PathBuf,
        branch: Option<String>,
    },
    WorktreeRemoved {
        workspace_id: String,
        path: PathBuf,
    },
    WorkspaceClosed {
        workspace_id: String,
    },
}

#[derive(Serialize)]
struct Request<'a> {
    id: &'static str,
    method: &'static str,
    params: Params<'a>,
}

#[derive(Serialize)]
struct Params<'a> {
    subscriptions: &'a [Subscription],
}

/// What a line may be: a response (`result` or `error`) or an event.
#[derive(Deserialize)]
struct Line {
    #[serde(default)]
    error: Option<ErrorBody>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    data: Value,
}

#[derive(Deserialize)]
struct WithPane {
    pane: Pane,
}

#[derive(Deserialize)]
struct PaneRef {
    pane_id: String,
    workspace_id: String,
}

#[derive(Deserialize)]
struct StatusChanged {
    pane_id: String,
    workspace_id: String,
    #[serde(default)]
    agent: Option<String>,
    agent_status: String,
}

#[derive(Deserialize)]
struct AgentDetected {
    pane_id: String,
    workspace_id: String,
    #[serde(default)]
    agent: Option<String>,
}

#[derive(Deserialize)]
struct WorkspaceRef {
    workspace_id: String,
}

#[derive(Deserialize)]
struct WorktreeRef {
    path: PathBuf,
    #[serde(default)]
    branch: Option<String>,
}

/// `worktree_created` and `worktree_opened`: the workspace is whole.
#[derive(Deserialize)]
struct WorktreeInWorkspace {
    workspace: WorkspaceRef,
    worktree: WorktreeRef,
}

/// `worktree_removed`: the workspace may be gone, its id stays.
#[derive(Deserialize)]
struct WorktreeGone {
    workspace_id: String,
    worktree: WorktreeRef,
}

impl Client {
    /// Opens the socket, subscribes, and waits for herdr's acknowledgement.
    /// An `error` answer is a [`HerdrError::Api`]. The stream ends when the
    /// socket closes or herdr sends an error; lines it cannot read are skipped.
    pub fn subscribe(
        &self,
        subscriptions: &[Subscription],
    ) -> Result<impl Iterator<Item = Event>, HerdrError> {
        let io = |context: &str| {
            let context = format!("{context} {}", self.socket.display());
            move |source| HerdrError::Io { context, source }
        };
        let mut stream = UnixStream::connect(&self.socket).map_err(io("connecting to"))?;
        let request = Request {
            id: "herdr-ank:subscribe",
            method: "events.subscribe",
            params: Params { subscriptions },
        };
        let mut line = serde_json::to_vec(&request)
            .map_err(|err| HerdrError::Decode(format!("encoding the request: {err}")))?;
        line.push(b'\n');
        stream.write_all(&line).map_err(io("writing to"))?;

        let mut lines = BufReader::new(stream).lines();
        let ack = match lines.next() {
            Some(line) => line.map_err(io("reading from"))?,
            None => {
                return Err(HerdrError::Decode(
                    "the socket closed before acknowledging events.subscribe".into(),
                ))
            }
        };
        match serde_json::from_str::<Line>(&ack) {
            Ok(Line {
                error: Some(body), ..
            }) => Err(body.into()),
            Ok(Line {
                result: Some(_), ..
            }) => Ok(EventStream { lines }),
            _ => Err(HerdrError::Decode(format!(
                "events.subscribe acknowledgement: {ack}"
            ))),
        }
    }
}

struct EventStream {
    lines: Lines<BufReader<UnixStream>>,
}

impl Iterator for EventStream {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            let line = self.lines.next()?.ok()?;
            let Ok(line) = serde_json::from_str::<Line>(&line) else {
                continue;
            };
            if line.error.is_some() {
                return None;
            }
            if let Some(event) = line
                .event
                .as_deref()
                .and_then(|kind| decode(kind, line.data))
            {
                return Some(event);
            }
        }
    }
}

/// herdr names a kind `pane.updated` in a subscription and `pane_updated` in
/// an event envelope; both spellings are read.
fn decode(kind: &str, data: Value) -> Option<Event> {
    match kind.replace('.', "_").as_str() {
        "pane_created" => from::<WithPane>(data).map(|d| Event::PaneCreated(d.pane)),
        "pane_updated" => from::<WithPane>(data).map(|d| Event::PaneUpdated(d.pane)),
        "pane_closed" => from::<PaneRef>(data).map(|d| Event::PaneClosed {
            pane_id: d.pane_id,
            workspace_id: d.workspace_id,
        }),
        "pane_agent_status_changed" => {
            from::<StatusChanged>(data).map(|d| Event::PaneAgentStatusChanged {
                pane_id: d.pane_id,
                workspace_id: d.workspace_id,
                agent: d.agent,
                agent_status: d.agent_status,
            })
        }
        "pane_agent_detected" => from::<AgentDetected>(data).map(|d| Event::PaneAgentDetected {
            pane_id: d.pane_id,
            workspace_id: d.workspace_id,
            agent: d.agent,
        }),
        "worktree_created" => from::<WorktreeInWorkspace>(data).map(|d| Event::WorktreeCreated {
            workspace_id: d.workspace.workspace_id,
            path: d.worktree.path,
            branch: d.worktree.branch,
        }),
        "worktree_opened" => from::<WorktreeInWorkspace>(data).map(|d| Event::WorktreeOpened {
            workspace_id: d.workspace.workspace_id,
            path: d.worktree.path,
            branch: d.worktree.branch,
        }),
        "worktree_removed" => from::<WorktreeGone>(data).map(|d| Event::WorktreeRemoved {
            workspace_id: d.workspace_id,
            path: d.worktree.path,
        }),
        "workspace_closed" => from::<WorkspaceRef>(data).map(|d| Event::WorkspaceClosed {
            workspace_id: d.workspace_id,
        }),
        _ => None,
    }
}

fn from<T: for<'de> Deserialize<'de>>(data: Value) -> Option<T> {
    serde_json::from_value(data).ok()
}
