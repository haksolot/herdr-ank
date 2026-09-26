//! `sync::plan` against SPEC-dbe3cf972f71 and ADR-fa8b6a103597: panes and
//! agents shaped like `herdr pane list` / `agent list`, corpora shaped like
//! `ank find --status in_progress --json` and `ank context --json`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use herdr_ank::ank::{Context, Find};
use herdr_ank::config::Config;
use herdr_ank::herdr::Pane;
use herdr_ank::sync::{plan, Corpus, Report};

const REPO: &str = "/src/repo";
const WT: &str = "/wt/agent-ank-2";

fn pane(id: &str, cwd: &str, tokens: &[(&str, &str)]) -> Pane {
    let tokens: BTreeMap<&str, &str> = tokens.iter().copied().collect();
    serde_json::from_value(serde_json::json!({
        "pane_id": id, "workspace_id": "w1", "tab_id": "w1:t1", "cwd": cwd,
        "focused": false, "revision": 1, "tokens": tokens,
    }))
    .unwrap()
}

fn agent(pane_id: &str, name: &str) -> Pane {
    serde_json::from_value(serde_json::json!({
        "pane_id": pane_id, "workspace_id": "w1", "tab_id": "w1:t1",
        "agent": "claude", "agent_status": "working", "name": name,
        "focused": false, "revision": 1,
    }))
    .unwrap()
}

/// `claims` is (task id, title, holder), as `find --status in_progress` lists them.
fn corpus(root: &str, claims: &[(&str, &str, &str)], ready: u64) -> Corpus {
    let results: Vec<_> = claims
        .iter()
        .map(|(id, title, holder)| {
            serde_json::json!({
                "id": id, "kind": "task", "status": "in_progress",
                "state": format!("claimed:{holder}"), "title": title,
                "created": "2026-09-25T16:54:25Z", "archived": false,
            })
        })
        .collect();
    let find: Find = serde_json::from_value(serde_json::json!({
        "contract": 1, "corpus": "110212bd017e", "total": results.len(),
        "shown": results.len(), "hidden": 0, "results": results,
    }))
    .unwrap();
    let context: Context = serde_json::from_value(serde_json::json!({
        "contract": 1, "mode": "orientation", "head": null, "criteria": null,
        "method": null, "constraints": [], "proposed": [], "specs": [],
        "tasks": [], "log": [], "ready": ready, "blocked": 0,
        "finished_elsewhere": 0, "warnings": [],
    }))
    .unwrap();
    Corpus {
        root: PathBuf::from(root),
        in_progress: find,
        context,
        expires_in: BTreeMap::new(),
    }
}

fn tokens(report: &Report) -> BTreeMap<&str, &str> {
    report
        .tokens
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}

fn clears(report: &Report) -> Vec<&str> {
    let mut clear: Vec<&str> = report.clear.iter().map(String::as_str).collect();
    clear.sort();
    clear
}

#[test]
fn a_claim_suffixed_with_an_agent_name_goes_to_that_agents_pane_only() {
    let panes = [
        pane("w1:p1", WT, &[]),
        pane("w1:p2", &format!("{WT}/src/herdr"), &[]),
    ];
    let agents = [agent("w1:p1", "ank-1"), agent("w1:p2", "ank-2")];
    let mut wt = corpus(
        WT,
        &[(
            "TASK-416cdde27bfb",
            "Moteur de synchronisation : des panes, worktrees et claims aux tokens de la spec",
            "haksolot@omarchy/ank-2",
        )],
        2,
    );
    wt.expires_in.insert("TASK-416cdde27bfb".into(), 42);

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert_eq!(reports.len(), 2, "{reports:#?}");
    let p2 = reports.iter().find(|r| r.pane_id == "w1:p2").unwrap();
    assert_eq!(p2.source, "ank:sync");
    assert_eq!(p2.ttl_ms, 90_000);
    assert_eq!(
        tokens(p2),
        BTreeMap::from([
            ("ank_task", "TASK-416c"),
            (
                "ank_title",
                "Moteur de synchronisation : des panes, worktrees et claims a"
            ),
            ("ank_expires", "42"),
            ("ank_queue", "2"),
        ])
    );
    assert_eq!(clears(p2), ["ank_ambiguous"]);

    let p1 = reports.iter().find(|r| r.pane_id == "w1:p1").unwrap();
    assert_eq!(tokens(p1), BTreeMap::from([("ank_queue", "2")]));
    assert_eq!(
        clears(p1),
        ["ank_ambiguous", "ank_expires", "ank_task", "ank_title"]
    );
}

#[test]
fn a_suffixed_claim_is_not_given_to_a_same_named_agent_in_another_worktree() {
    let panes = [pane("w1:p1", REPO, &[]), pane("w1:p2", WT, &[])];
    let agents = [agent("w1:p1", "ank-2"), agent("w1:p2", "ank-2")];
    let repo = corpus(REPO, &[], 0);
    let wt = corpus(
        WT,
        &[("TASK-de0a51842971", "Client herdr", "me@host/ank-2")],
        0,
    );

    let reports = plan(&panes, &agents, &[repo, wt], &Config::default());

    let in_repo = reports.iter().find(|r| r.pane_id == "w1:p1").unwrap();
    assert!(!tokens(in_repo).contains_key("ank_task"), "{in_repo:#?}");
    let in_wt = reports.iter().find(|r| r.pane_id == "w1:p2").unwrap();
    assert_eq!(tokens(in_wt).get("ank_task"), Some(&"TASK-de0a"));
}

#[test]
fn a_fallback_identity_claim_on_two_panes_of_one_worktree_is_ambiguous() {
    let panes = [pane("w1:p1", WT, &[]), pane("w1:p2", WT, &[])];
    let agents = [agent("w1:p1", "left"), agent("w1:p2", "right")];
    let wt = corpus(
        WT,
        &[("TASK-8e7f97898a70", "Pane ank tui", "haksolot@omarchy")],
        1,
    );

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert_eq!(reports.len(), 2);
    for report in &reports {
        let t = tokens(report);
        assert_eq!(t.get("ank_task"), Some(&"TASK-8e7f"), "{report:#?}");
        assert_eq!(t.get("ank_ambiguous"), Some(&"1"), "{report:#?}");
    }
}

#[test]
fn a_fallback_identity_claim_on_a_single_agent_pane_is_not_ambiguous() {
    let panes = [pane("w1:p1", WT, &[])];
    let agents = [agent("w1:p1", "solo")];
    let wt = corpus(
        WT,
        &[("TASK-8e7f97898a70", "Pane ank tui", "haksolot@omarchy")],
        1,
    );

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert_eq!(tokens(&reports[0]).get("ank_task"), Some(&"TASK-8e7f"));
    assert!(clears(&reports[0]).contains(&"ank_ambiguous"));
}

#[test]
fn a_pane_without_agent_or_outside_every_corpus_receives_nothing() {
    let panes = [
        pane("w1:p1", WT, &[]),           // no agent
        pane("w1:p2", "/elsewhere", &[]), // agent, no corpus
        pane("w1:p3", "/src/repository-sibling", &[]),
    ];
    let agents = [agent("w1:p2", "ank-2"), agent("w1:p3", "ank-3")];
    let wt = corpus(WT, &[("TASK-416cdde27bfb", "t", "me@host/ank-2")], 3);
    let repo = corpus(REPO, &[("TASK-de0a51842971", "t", "me@host/ank-3")], 3);

    assert!(plan(&panes, &agents, &[wt, repo], &Config::default()).is_empty());
}

#[test]
fn a_vanished_claim_clears_the_claim_tokens_and_keeps_the_queue() {
    let stale = [
        ("ank_task", "TASK-416c"),
        ("ank_title", "t"),
        ("ank_expires", "3"),
        ("ank_queue", "2"),
    ];
    let panes = [pane("w1:p2", WT, &stale)];
    let agents = [agent("w1:p2", "ank-2")];

    let reports = plan(&panes, &agents, &[corpus(WT, &[], 1)], &Config::default());

    assert_eq!(tokens(&reports[0]), BTreeMap::from([("ank_queue", "1")]));
    assert_eq!(
        clears(&reports[0]),
        ["ank_ambiguous", "ank_expires", "ank_task", "ank_title"]
    );
}

#[test]
fn a_claim_gone_with_its_corpus_clears_all_five_tokens() {
    let stale = [
        ("ank_task", "TASK-416c"),
        ("ank_title", "t"),
        ("ank_expires", "3"),
        ("ank_ambiguous", "1"),
        ("ank_queue", "2"),
        ("other_plugin", "kept"),
    ];
    let panes = [pane("w1:p2", WT, &stale)];
    let agents = [agent("w1:p2", "ank-2")];

    let reports = plan(&panes, &agents, &[], &Config::default());

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].pane_id, "w1:p2");
    assert!(reports[0].tokens.is_empty());
    assert_eq!(
        clears(&reports[0]),
        [
            "ank_ambiguous",
            "ank_expires",
            "ank_queue",
            "ank_task",
            "ank_title"
        ]
    );
}

#[test]
fn the_deepest_corpus_root_owns_the_pane() {
    let nested = "/src/repo/.herdr/worktrees/ank-2";
    let panes = [pane("w1:p2", &format!("{nested}/src"), &[])];
    let agents = [agent("w1:p2", "ank-2")];
    let outer = corpus(REPO, &[], 7);
    let inner = corpus(nested, &[], 1);

    let reports = plan(&panes, &agents, &[outer, inner], &Config::default());

    assert_eq!(tokens(&reports[0]).get("ank_queue"), Some(&"1"));
}

#[test]
fn a_fallback_claim_is_ambiguous_only_across_the_panes_it_is_shown_on() {
    let panes = [pane("w1:p1", WT, &[]), pane("w1:p2", WT, &[])];
    let agents = [agent("w1:p1", "ank-1"), agent("w1:p2", "other")];
    let wt = corpus(
        WT,
        &[
            ("TASK-8e7f97898a70", "Pane ank tui", "haksolot@omarchy"),
            ("TASK-c81aba215f4d", "Action work", "haksolot@omarchy/ank-1"),
        ],
        0,
    );

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    let named = reports.iter().find(|r| r.pane_id == "w1:p1").unwrap();
    assert_eq!(tokens(named).get("ank_task"), Some(&"TASK-c81a"));
    let bare = reports.iter().find(|r| r.pane_id == "w1:p2").unwrap();
    assert_eq!(tokens(bare).get("ank_task"), Some(&"TASK-8e7f"));
    assert_eq!(tokens(bare).get("ank_ambiguous"), None, "{bare:#?}");
}

/// A pane as `pane list` shows one hosting agent `kind` (its `agent` field).
fn agent_pane(id: &str, cwd: &str, kind: &str) -> Pane {
    serde_json::from_value(serde_json::json!({
        "pane_id": id, "workspace_id": "w1", "tab_id": "w1:t1", "cwd": cwd,
        "agent": kind, "agent_status": "working", "focused": false, "revision": 1,
    }))
    .unwrap()
}

#[test]
fn an_agent_pane_holding_a_claim_is_labelled_with_its_agent_and_the_short_id() {
    let panes = [agent_pane("w1:p2", WT, "claude")];
    let agents = [agent("w1:p2", "ank-2")];
    let wt = corpus(
        WT,
        &[("TASK-416cdde27bfb", "Sync", "haksolot@omarchy/ank-2")],
        2,
    );

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert_eq!(reports.len(), 1, "{reports:#?}");
    assert_eq!(
        reports[0].display_agent.as_deref(),
        Some("claude · TASK-416c")
    );
    assert_eq!(reports[0].ttl_ms, 90_000);
    assert_eq!(tokens(&reports[0]).get("ank_task"), Some(&"TASK-416c"));
}

#[test]
fn an_agent_pane_in_a_corpus_without_its_claim_is_labelled_ank() {
    let panes = [agent_pane("w1:p1", WT, "codex")];
    let agents = [agent("w1:p1", "ank-1")];
    // the claim belongs to ank-2: the label reads herdr's agent name, and
    // ank-1 is not ank-2 whatever the pane's label says
    let wt = corpus(
        WT,
        &[("TASK-416cdde27bfb", "Sync", "haksolot@omarchy/ank-2")],
        2,
    );

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert_eq!(reports.len(), 1, "{reports:#?}");
    assert_eq!(reports[0].display_agent.as_deref(), Some("codex · ank"));
    assert!(!tokens(&reports[0]).contains_key("ank_task"));
}

#[test]
fn no_label_outside_every_corpus_or_without_an_agent_field() {
    let panes = [
        agent_pane("w1:p1", "/elsewhere", "claude"),
        pane("w1:p2", WT, &[]),
    ];
    let agents = [agent("w1:p1", "ank-1"), agent("w1:p2", "ank-2")];
    let wt = corpus(WT, &[], 2);

    let reports = plan(&panes, &agents, &[wt], &Config::default());

    assert!(reports.iter().all(|r| r.pane_id != "w1:p1"), "{reports:#?}");
    let p2 = reports.iter().find(|r| r.pane_id == "w1:p2").unwrap();
    assert_eq!(p2.display_agent, None);
}
