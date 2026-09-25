//! What the daemon tells the user between two synchronisations: a task a pane
//! held is done, a claim is about to expire, decisions wait for ratification.
//!
//! The previous state lives in memory only: a restarted daemon may tell one
//! thing a second time, which is accepted.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::herdr::{Pane, Sound};
use crate::sync::Corpus;

/// One `herdr notification show`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub body: Option<String>,
    pub sound: Sound,
}

/// A claim attributed to one agent pane: its holder ends with `/<name>` and
/// an agent of that name sits in the corpus (ADR-fa8b6a103597). A claim held
/// by the bare fallback identity is attributed to no single pane.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Held {
    pub root: PathBuf,
    pub id: String,
    pub holder: String,
    pub agent: String,
    pub expires_in: Option<u64>,
}

impl Held {
    /// What makes it the same claim from one synchronisation to the next.
    fn key(&self) -> (PathBuf, String, String) {
        (self.root.clone(), self.id.clone(), self.holder.clone())
    }
}

/// What one synchronisation saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    pub held: Vec<Held>,
    /// `queue` of `ank status --json`, by corpus root.
    pub queues: BTreeMap<PathBuf, u64>,
}

impl Observed {
    pub fn from_sync(
        panes: &[Pane],
        agents: &[Pane],
        corpora: &[Corpus],
        queues: &BTreeMap<PathBuf, u64>,
    ) -> Self {
        let mut held = Vec::new();
        for corpus in corpora {
            // Names of the agents whose pane sits in this corpus.
            let names: BTreeSet<&str> = panes
                .iter()
                .filter(|p| {
                    p.cwd
                        .as_deref()
                        .is_some_and(|cwd| cwd.starts_with(&corpus.root))
                })
                .filter_map(|p| agents.iter().find(|a| a.pane_id == p.pane_id))
                .filter_map(|a| a.name.as_deref())
                .collect();
            for found in &corpus.in_progress.results {
                let Some(holder) = found.state.strip_prefix("claimed:") else {
                    continue;
                };
                let Some((_, agent)) = holder.rsplit_once('/') else {
                    continue;
                };
                if names.contains(agent) {
                    held.push(Held {
                        root: corpus.root.clone(),
                        id: found.id.clone(),
                        holder: holder.to_owned(),
                        agent: agent.to_owned(),
                        expires_in: corpus.expires_in.get(&found.id).copied(),
                    });
                }
            }
        }
        held.sort();
        Observed {
            held,
            queues: queues.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Notifier {
    previous: Option<Observed>,
    /// Claims already told as expiring, forgotten once the claim is gone.
    told_expiring: BTreeSet<(PathBuf, String, String)>,
}

impl Notifier {
    /// Compares `now` with the previous synchronisation and says what to
    /// tell. `is_done(root, id)` is asked only of a claim that disappeared.
    pub fn observe(
        &mut self,
        now: Observed,
        config: &Config,
        is_done: impl Fn(&Path, &str) -> bool,
    ) -> Vec<Notification> {
        let mut told = Vec::new();
        let live: BTreeSet<_> = now.held.iter().map(Held::key).collect();

        if let Some(previous) = &self.previous {
            if config.notify.done {
                for gone in previous.held.iter().filter(|h| !live.contains(&h.key())) {
                    if is_done(&gone.root, &gone.id) {
                        told.push(Notification {
                            title: format!("{} terminée par {}", short_id(&gone.id), gone.agent),
                            body: None,
                            sound: Sound::Done,
                        });
                    }
                }
            }
        }

        self.told_expiring.retain(|key| live.contains(key));
        for held in &now.held {
            let Some(minutes) = held.expires_in else {
                continue;
            };
            if minutes < config.notify.expiring_minutes && self.told_expiring.insert(held.key()) {
                told.push(Notification {
                    title: format!("claim {} expire dans {minutes} min", short_id(&held.id)),
                    body: None,
                    sound: Sound::Request,
                });
            }
        }

        if config.notify.review {
            for (root, &queue) in &now.queues {
                let before = self
                    .previous
                    .as_ref()
                    .and_then(|p| p.queues.get(root).copied())
                    .unwrap_or(0);
                if queue > 0 && queue > before {
                    told.push(Notification {
                        title: format!("{queue} décisions en attente"),
                        body: Some("ank review".to_owned()),
                        sound: Sound::None,
                    });
                }
            }
        }

        self.previous = Some(now);
        told
    }
}

/// `TASK-416cdde27bfb` -> `TASK-416c`, as the pane tokens show it.
fn short_id(id: &str) -> String {
    match id.split_once('-') {
        Some((kind, hash)) => format!("{kind}-{}", hash.chars().take(4).collect::<String>()),
        None => id.to_owned(),
    }
}
