//! From herdr's panes and ank's claims to the pane tokens of
//! SPEC-43438bbcb5ca. `plan` is pure: the clients fetch, it decides.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ank::{Context, Find};
use crate::config::Config;
use crate::herdr::Pane;

/// The `--source` of every report (SPEC-43438bbcb5ca).
pub const SOURCE: &str = "ank:sync";

pub const TASK: &str = "ank_task";
pub const TITLE: &str = "ank_title";
pub const EXPIRES: &str = "ank_expires";
pub const AMBIGUOUS: &str = "ank_ambiguous";
pub const QUEUE: &str = "ank_queue";
/// Workspace tokens (SPEC-43438bbcb5ca), beside `QUEUE`.
pub const CLAIMS: &str = "ank_claims";
pub const REVIEW: &str = "ank_review";

/// Every token the plugin owns, in the order the spec lists them.
pub const TOKENS: [&str; 5] = [TASK, TITLE, EXPIRES, AMBIGUOUS, QUEUE];

/// `--ttl-ms` of every report, fixed by SPEC-43438bbcb5ca.
pub const TTL_MS: u64 = 90_000;

const TITLE_CHARS: usize = 60;

/// Between the agent and what ank adds to its label: space, U+00B7, space.
const LABEL_SEPARATOR: &str = " \u{b7} ";

/// One worktree carrying a corpus, as its clients saw it.
#[derive(Debug, Clone)]
pub struct Corpus {
    /// The worktree root: `find` and `context` ran with `--repo` on it.
    pub root: PathBuf,
    /// `ank find --status in_progress --json`.
    pub in_progress: Find,
    /// `ank context --json`, whose `ready` is the claimable count.
    pub context: Context,
    /// Minutes before each live claim expires, by task id. Neither `find`
    /// nor `context` carries an expiry; the fetching side reads it from
    /// `ank status --json`. A claim missing here reports no `ank_expires`.
    pub expires_in: BTreeMap<String, u64>,
}

/// One `herdr workspace report-metadata` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReport {
    pub workspace_id: String,
    pub source: &'static str,
    pub tokens: Vec<(String, String)>,
    pub ttl_ms: u64,
}

/// One `herdr pane report-metadata` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub pane_id: String,
    /// `--display-agent`: `<agent> · <short id>` or `<agent> · ank`.
    pub display_agent: Option<String>,
    pub source: &'static str,
    pub tokens: Vec<(String, String)>,
    pub clear: Vec<String>,
    pub ttl_ms: u64,
}

/// A live claim as `find` shows it: `state` is `claimed:<holder>`.
struct Claim<'a> {
    id: &'a str,
    title: &'a str,
    /// The herdr agent name after the last `/`, or `None` for the bare
    /// fallback identity (ADR-fa8b6a103597).
    agent: Option<&'a str>,
}

/// The reports that bring every pane's tokens in line with the corpora.
///
/// A pane hosting an agent (one of `agents`) inside a corpus gets the full
/// token set, every absent token cleared. A pane that no longer qualifies
/// but still shows one of the plugin's tokens gets them all cleared. Any
/// other pane gets nothing.
///
/// `config` is part of the signature the engine was planned with; no rule
/// of the spec reads it yet (the ttl is fixed, not derived from the poll).
pub fn plan(panes: &[Pane], agents: &[Pane], corpora: &[Corpus], _config: &Config) -> Vec<Report> {
    let ttl_ms = TTL_MS;
    let agent_name = |pane: &Pane| -> Option<&str> {
        agents
            .iter()
            .find(|a| a.pane_id == pane.pane_id)
            .map(|a| a.name.as_deref().unwrap_or(""))
    };

    // Each agent pane with the corpus that owns its cwd.
    let placed: Vec<(&Pane, &str, &Corpus)> = panes
        .iter()
        .filter_map(|pane| {
            let name = agent_name(pane)?;
            let corpus = owner(pane.cwd.as_deref()?, corpora)?;
            Some((pane, name, corpus))
        })
        .collect();

    let mut reports = Vec::new();
    for pane in panes {
        let Some(&(_, name, corpus)) = placed.iter().find(|(p, ..)| p.pane_id == pane.pane_id)
        else {
            if TOKENS.iter().any(|t| pane.tokens.contains_key(*t)) {
                reports.push(Report {
                    pane_id: pane.pane_id.clone(),
                    display_agent: None,
                    source: SOURCE,
                    tokens: Vec::new(),
                    clear: TOKENS.iter().map(|t| t.to_string()).collect(),
                    ttl_ms,
                });
            }
            continue;
        };

        let mut set = BTreeMap::new();
        let claim = attributed(corpus, name);
        // The label shows herdr's `agent` kind; attribution above read the
        // agent's name, never this label (SPEC-43438bbcb5ca).
        let display_agent = pane
            .agent
            .as_deref()
            .filter(|kind| !kind.is_empty())
            .map(|kind| match &claim {
                Some(claim) => format!("{kind}{LABEL_SEPARATOR}{}", short_id(claim.id)),
                None => format!("{kind}{LABEL_SEPARATOR}ank"),
            });
        if let Some(claim) = claim {
            set.insert(TASK, short_id(claim.id));
            set.insert(TITLE, claim.title.chars().take(TITLE_CHARS).collect());
            if let Some(minutes) = corpus.expires_in.get(claim.id) {
                set.insert(EXPIRES, minutes.to_string());
            }
            let shown_on = placed
                .iter()
                .filter(|(_, other_name, other)| {
                    std::ptr::eq(*other, corpus)
                        && attributed(other, other_name).is_some_and(|c| c.id == claim.id)
                })
                .count();
            if claim.agent.is_none() && shown_on > 1 {
                set.insert(AMBIGUOUS, "1".to_owned());
            }
        }
        set.insert(QUEUE, corpus.context.ready.to_string());

        reports.push(Report {
            pane_id: pane.pane_id.clone(),
            display_agent,
            source: SOURCE,
            clear: TOKENS
                .iter()
                .filter(|t| !set.contains_key(*t))
                .map(|t| t.to_string())
                .collect(),
            tokens: TOKENS
                .iter()
                .filter_map(|t| Some((t.to_string(), set.remove(t)?)))
                .collect(),
            ttl_ms,
        });
    }
    reports
}

/// The corpus whose root is the deepest ancestor of `cwd`.
fn owner<'a>(cwd: &Path, corpora: &'a [Corpus]) -> Option<&'a Corpus> {
    corpora
        .iter()
        .filter(|c| cwd.starts_with(&c.root))
        .max_by_key(|c| c.root.components().count())
}

/// The claim a pane whose agent is `name` shows: the one suffixed with
/// `/<name>`, else a bare fallback-identity claim of its worktree.
fn attributed<'a>(corpus: &'a Corpus, name: &str) -> Option<Claim<'a>> {
    claims(corpus).find(|c| c.agent.is_none_or(|agent| agent == name))
}

/// The live claims of a corpus, suffixed identities first so a named agent's
/// claim wins over a bare one in the same worktree, then by id.
fn claims(corpus: &Corpus) -> impl Iterator<Item = Claim<'_>> {
    let mut claims: Vec<Claim<'_>> = corpus
        .in_progress
        .results
        .iter()
        .filter_map(|found| {
            let holder = found.state.strip_prefix("claimed:")?;
            Some(Claim {
                id: &found.id,
                title: &found.title,
                agent: holder.rsplit_once('/').map(|(_, name)| name),
            })
        })
        .collect();
    claims.sort_by_key(|c| (c.agent.is_none(), c.id));
    claims.into_iter()
}

/// `TASK-416cdde27bfb` -> `TASK-416c`.
fn short_id(id: &str) -> String {
    match id.split_once('-') {
        Some((kind, hash)) => format!("{kind}-{}", hash.chars().take(4).collect::<String>()),
        None => id.to_owned(),
    }
}

/// One report per workspace with at least one pane inside a corpus, any
/// pane, agent or not: the counts of the corpus owning most of its panes,
/// and on a tie the corpus of the first such pane in `panes` order. The
/// three tokens are always sent, `0` included; no other workspace is
/// reported (SPEC-43438bbcb5ca).
///
/// `reviews` holds, by corpus root, the `queue` of `ank status --json`: the
/// decisions awaiting ratification, what `ank review` lists, read without
/// running it (ADR-3cd19cd6acb9). A root missing there counts `0`.
pub fn plan_workspaces(
    panes: &[Pane],
    corpora: &[Corpus],
    reviews: &BTreeMap<PathBuf, u64>,
) -> Vec<WorkspaceReport> {
    // Per workspace, in first-seen order: each corpus with its pane count,
    // in the order its first pane appears.
    let mut seen: Vec<(&str, Vec<(&Corpus, usize)>)> = Vec::new();
    for pane in panes {
        let Some(corpus) = pane.cwd.as_deref().and_then(|cwd| owner(cwd, corpora)) else {
            continue;
        };
        let workspace = pane.workspace_id.as_str();
        let counts = match seen.iter_mut().find(|(w, _)| *w == workspace) {
            Some((_, counts)) => counts,
            None => {
                seen.push((workspace, Vec::new()));
                &mut seen.last_mut().expect("just pushed").1
            }
        };
        match counts.iter_mut().find(|(c, _)| std::ptr::eq(*c, corpus)) {
            Some((_, n)) => *n += 1,
            None => counts.push((corpus, 1)),
        }
    }

    seen.into_iter()
        .filter_map(|(workspace, counts)| {
            // max_by_key keeps the last maximum: reverse so the first wins.
            let (corpus, _) = counts.into_iter().rev().max_by_key(|(_, n)| *n)?;
            let claims = corpus
                .in_progress
                .results
                .iter()
                .filter(|found| found.state.starts_with("claimed:"))
                .count();
            Some(WorkspaceReport {
                workspace_id: workspace.to_owned(),
                source: SOURCE,
                tokens: vec![
                    (QUEUE.to_owned(), corpus.context.ready.to_string()),
                    (CLAIMS.to_owned(), claims.to_string()),
                    (
                        REVIEW.to_owned(),
                        reviews.get(&corpus.root).copied().unwrap_or(0).to_string(),
                    ),
                ],
                ttl_ms: TTL_MS,
            })
        })
        .collect()
}
