//! The documents of contract 1, field for field as `ank help --json` publishes
//! them: a nullable field is an `Option`, and a field this crate does not
//! declare is ignored.

use serde::Deserialize;

/// `ank status --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    pub corpus: Option<String>,
    /// `None` on a detached HEAD.
    pub branch: Option<String>,
    /// `None` when the default branch cannot be determined.
    pub default_branch: Option<String>,
    pub identity: Identity,
    pub claim: Option<HeldClaim>,
    pub drift: Option<Drift>,
    pub also_held: Vec<AlsoHeld>,
    pub remote: bool,
    pub refs: Option<Refs>,
    pub elsewhere: Vec<Elsewhere>,
    pub constraints: u64,
    pub queue: u64,
    pub unmerged: u64,
    pub faults: u64,
    pub signals: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Identity {
    pub value: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeldClaim {
    pub id: String,
    pub expires: String,
    pub lapsed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drift {
    pub branch: String,
    pub entities: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlsoHeld {
    pub id: String,
    pub expires: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Refs {
    pub stale: u64,
    pub absent: u64,
}

/// A claim held by another agent.
#[derive(Debug, Clone, Deserialize)]
pub struct Elsewhere {
    pub id: String,
    pub title: Option<String>,
    pub holder: Option<String>,
    pub expires: Option<String>,
    pub seen: Option<String>,
}

/// `ank find --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Find {
    pub corpus: Option<String>,
    pub total: u64,
    pub shown: u64,
    pub hidden: u64,
    pub results: Vec<Found>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Found {
    pub id: String,
    pub kind: String,
    pub status: String,
    /// The displayed state: `open`, or `claimed:<holder>` for a live claim.
    pub state: String,
    pub title: String,
    pub created: String,
    pub archived: bool,
}

/// `ank show --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Show {
    pub id: String,
    pub coordination: Option<String>,
    #[serde(default)]
    pub blocked_by: Vec<Link>,
    #[serde(default)]
    pub unblocks: Vec<Link>,
    #[serde(default)]
    pub detached_proofs: Vec<DetachedProof>,
    #[serde(default)]
    pub log_total: u64,
    #[serde(default)]
    pub log_shown: u64,
    #[serde(default)]
    pub log: Vec<LogEntry>,
    #[serde(default)]
    pub machinery: Vec<LogEntry>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Link {
    pub id: String,
    pub short: String,
    pub status: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetachedProof {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub by: String,
    pub at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogEntry {
    pub id: Option<String>,
    pub timestamp: String,
    pub who: String,
    pub message: String,
    pub records: Option<String>,
}

/// `ank context --json`, without `--since`.
#[derive(Debug, Clone, Deserialize)]
pub struct Context {
    pub mode: String,
    pub head: Option<String>,
    pub criteria: Option<String>,
    pub method: Option<String>,
    pub constraints: Vec<Constraint>,
    pub proposed: Vec<Proposed>,
    pub specs: Vec<Spec>,
    /// The tasks of this perimeter; `ready` marks the claimable ones.
    pub tasks: Vec<ContextTask>,
    pub log: Vec<serde_json::Value>,
    pub ready: u64,
    pub blocked: u64,
    pub finished_elsewhere: u64,
    pub warnings: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub short: String,
    pub title: String,
    pub constraint: String,
    pub home: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Proposed {
    pub id: String,
    pub short: String,
    pub title: String,
    pub home: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    pub id: String,
    pub short: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContextTask {
    pub id: String,
    pub short: String,
    pub title: String,
    pub status: String,
    pub ready: bool,
    pub unblocks: u64,
    pub state: String,
}
