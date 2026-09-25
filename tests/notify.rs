//! The daemon's notifications: what changed between two synchronisations,
//! told once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use herdr_ank::ank::{Context, Find};
use herdr_ank::config::Config;
use herdr_ank::herdr::{Pane, Sound};
use herdr_ank::notify::{Notification, Notifier, Observed};
use herdr_ank::sync::Corpus;

const ROOT: &str = "/src/repo";

fn pane(id: &str, cwd: &str, name: Option<&str>) -> Pane {
    serde_json::from_value(serde_json::json!({
        "pane_id": id, "workspace_id": "w1", "tab_id": "w1:t1",
        "cwd": cwd, "name": name,
    }))
    .unwrap()
}

fn find(claims: &[(&str, &str)]) -> Find {
    let results: Vec<_> = claims
        .iter()
        .map(|(id, holder)| {
            serde_json::json!({
                "id": id, "kind": "task", "status": "in_progress",
                "state": format!("claimed:{holder}"), "title": "A task",
                "created": "2026-09-25T16:54:28Z", "archived": false,
            })
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "contract": 1, "corpus": null, "total": results.len(),
        "shown": results.len(), "hidden": 0, "results": results,
    }))
    .unwrap()
}

fn context() -> Context {
    serde_json::from_value(serde_json::json!({
        "contract": 1, "mode": "orientation", "head": null, "criteria": null,
        "method": null, "constraints": [], "proposed": [], "specs": [], "tasks": [],
        "log": [], "ready": 0, "blocked": 0, "finished_elsewhere": 0, "warnings": [],
    }))
    .unwrap()
}

/// One synchronisation: agent panes `ank-2` in the repo and `ank-9` outside
/// any corpus, the given claims with their minutes left, and the queue.
fn sync(claims: &[(&str, &str, Option<u64>)], queue: u64) -> Observed {
    let panes = [
        pane("w1:p2", "/src/repo/sub", Some("ank-2")),
        pane("w1:p9", "/elsewhere", Some("ank-9")),
        pane("w1:p1", "/src/repo", None),
    ];
    let agents: Vec<Pane> = panes.iter().filter(|p| p.name.is_some()).cloned().collect();
    let corpus = Corpus {
        root: PathBuf::from(ROOT),
        in_progress: find(
            &claims
                .iter()
                .map(|(id, holder, _)| (*id, *holder))
                .collect::<Vec<_>>(),
        ),
        context: context(),
        expires_in: claims
            .iter()
            .filter_map(|(id, _, minutes)| Some((id.to_string(), (*minutes)?)))
            .collect(),
    };
    let queues = BTreeMap::from([(PathBuf::from(ROOT), queue)]);
    Observed::from_sync(&panes, &agents, &[corpus], &queues)
}

fn never_done(_: &Path, _: &str) -> bool {
    false
}

fn always_done(_: &Path, _: &str) -> bool {
    true
}

const ID: &str = "TASK-416cdde27bfb";
const MINE: &str = "marie@box/ank-2";

#[test]
fn a_claim_held_by_a_pane_that_goes_done_is_told_once_with_the_agent_name() {
    let config = Config::default();
    let mut notifier = Notifier::default();
    assert!(notifier
        .observe(sync(&[(ID, MINE, Some(50))], 0), &config, always_done)
        .is_empty());
    let told = notifier.observe(sync(&[], 0), &config, always_done);
    assert_eq!(
        told,
        [Notification {
            title: "TASK-416c terminée par ank-2".into(),
            body: None,
            sound: Sound::Done,
        }]
    );
    assert!(notifier
        .observe(sync(&[], 0), &config, always_done)
        .is_empty());
}

#[test]
fn a_claim_that_vanishes_without_done_is_not_a_completion() {
    let config = Config::default();
    let mut notifier = Notifier::default();
    notifier.observe(sync(&[(ID, MINE, Some(50))], 0), &config, never_done);
    assert!(notifier
        .observe(sync(&[], 0), &config, never_done)
        .is_empty());
}

#[test]
fn only_a_claim_attributed_to_a_pane_counts() {
    let config = Config::default();
    let mut notifier = Notifier::default();
    // A bare fallback identity is never attributed to one pane (ADR-fa8b),
    // and ank-7 has no pane in this corpus.
    notifier.observe(
        sync(
            &[
                (ID, "marie@box", Some(5)),
                ("TASK-7777aaaa0000", "jo@x/ank-7", Some(5)),
            ],
            0,
        ),
        &config,
        always_done,
    );
    let told = notifier.observe(sync(&[], 0), &config, always_done);
    assert!(told.is_empty(), "{told:?}");
}

#[test]
fn an_expiring_claim_is_told_once_per_claim() {
    let config = Config::default(); // notify.expiring_minutes = 10
    let mut notifier = Notifier::default();
    assert!(notifier
        .observe(sync(&[(ID, MINE, Some(10))], 0), &config, never_done)
        .is_empty());
    assert_eq!(
        notifier.observe(sync(&[(ID, MINE, Some(9))], 0), &config, never_done),
        [Notification {
            title: "claim TASK-416c expire dans 9 min".into(),
            body: None,
            sound: Sound::Request,
        }]
    );
    assert!(notifier
        .observe(sync(&[(ID, MINE, Some(8))], 0), &config, never_done)
        .is_empty());
    // Renewed, then running low again: still the same claim, told already.
    notifier.observe(sync(&[(ID, MINE, Some(60))], 0), &config, never_done);
    assert!(notifier
        .observe(sync(&[(ID, MINE, Some(3))], 0), &config, never_done)
        .is_empty());
}

#[test]
fn a_new_claim_on_the_same_task_is_told_again() {
    let config = Config::default();
    let mut notifier = Notifier::default();
    assert_eq!(
        notifier
            .observe(sync(&[(ID, MINE, Some(4))], 0), &config, never_done)
            .len(),
        1
    );
    notifier.observe(sync(&[], 0), &config, never_done);
    assert_eq!(
        notifier
            .observe(sync(&[(ID, MINE, Some(4))], 0), &config, never_done)
            .len(),
        1
    );
}

#[test]
fn a_growing_ratification_queue_is_told_and_a_steady_one_is_not() {
    let config = Config::default();
    let mut notifier = Notifier::default();
    let queued = |n: u64| Notification {
        title: format!("{n} décisions en attente"),
        body: Some("ank review".into()),
        sound: Sound::None,
    };
    assert!(notifier
        .observe(sync(&[], 0), &config, never_done)
        .is_empty());
    assert_eq!(
        notifier.observe(sync(&[], 2), &config, never_done),
        [queued(2)]
    );
    assert!(notifier
        .observe(sync(&[], 2), &config, never_done)
        .is_empty());
    assert!(notifier
        .observe(sync(&[], 1), &config, never_done)
        .is_empty());
    assert_eq!(
        notifier.observe(sync(&[], 3), &config, never_done),
        [queued(3)]
    );
}

#[test]
fn every_rule_is_switched_off_by_its_config_key() {
    let mut config = Config::default();
    config.notify.done = false;
    config.notify.review = false;
    config.notify.expiring_minutes = 0;
    let mut notifier = Notifier::default();
    let told: Vec<_> = [sync(&[(ID, MINE, Some(1))], 4), sync(&[], 9)]
        .into_iter()
        .flat_map(|observed| notifier.observe(observed, &config, always_done))
        .collect();
    assert!(told.is_empty(), "{told:?}");
}
