//! The "work a task" action, end to end on a fake `herdr` that records every
//! call in order and plays the picker, and a fake `ank` that answers from
//! documents the test writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use herdr_ank::ank::ContextTask;
use herdr_ank::config::Config;
use herdr_ank::herdr;
use herdr_ank::pick;
use herdr_ank::work::{self, Outcome, Work, WorkError};

mod support;

fn scratch(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "work-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The fake `ank` (tests/support/fake_cli.rs), shared: its home is the parent
/// of the `--repo <dir>/repo` it is handed, and `setup` writes its answers
/// there, one per verb.
fn fake_ank() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = scratch("ank-bin");
        support::home_from_last_arg(&dir);
        support::link(&dir, "ank")
    })
}

const PANE: &str = r#"{"pane_id":"PANE","workspace_id":"w1","tab_id":"w1:t9","agent_status":"unknown","focused":true,"revision":1}"#;

/// Where the fake `herdr` of a test lives and records its calls, apart from
/// the fake `ank`'s home.
fn herdr_home(dir: &Path) -> PathBuf {
    dir.join("herdr-home")
}

/// A fake `herdr` for one test: records each call, answers each verb as herdr
/// 0.9.1 does, and on `plugin pane open` plays the picker by copying `pick`
/// (the chosen id, or nothing for a cancel) into the state directory.
fn fake_herdr(dir: &Path, state: &Path, pick: &str) -> herdr::Client {
    let home = herdr_home(dir);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("pick"), pick).unwrap();
    let bin = support::link(&home, "herdr");
    let pane = |id: &str| PANE.replace("PANE", id);
    let mut picked = support::answer(
        &["plugin", "pane"],
        &format!(
            r#"{{"id":"cli:plugin","result":{{"type":"plugin_pane_opened","plugin_pane":{{"plugin_id":"ank","entrypoint":"pick","pane":{}}}}}}}"#,
            pane("w1:p-pick")
        ),
        "",
        0,
    );
    picked["copy"] = serde_json::json!([home.join("pick"), state.join("pick")]);
    let answer = |args: &[&str], result: String| support::answer(args, &result, "", 0);
    support::write_answers(
        &home,
        &[
            picked,
            answer(
                &["worktree", "create"],
                format!(
                    r#"{{"id":"cli","result":{{"type":"worktree_created","root_pane":{}}}}}"#,
                    pane("w1:p-wt")
                ),
            ),
            answer(
                &["tab", "create"],
                format!(
                    r#"{{"id":"cli","result":{{"type":"tab_created","root_pane":{}}}}}"#,
                    pane("w1:p-tab")
                ),
            ),
            answer(
                &["agent", "start"],
                format!(
                    r#"{{"id":"cli","result":{{"type":"agent_started","agent":{},"argv":[]}}}}"#,
                    pane("w1:p-tab")
                ),
            ),
            answer(
                &["agent", "prompt"],
                format!(
                    r#"{{"id":"cli","result":{{"type":"agent_prompted","agent":{}}}}}"#,
                    pane("w1:p-tab")
                ),
            ),
            answer(&[], r#"{"id":"cli","result":{"type":"ok"}}"#.into()),
        ],
    );
    herdr::Client::new(bin, dir.join("herdr.sock"))
}

/// Each herdr call, its arguments joined by `|`.
fn calls(dir: &Path) -> Vec<String> {
    support::calls(&herdr_home(dir))
        .into_iter()
        .map(|call| call.argv.join("|"))
        .collect()
}

const STATUS: &str = r#"{"contract":1,"corpus":null,"branch":"main","default_branch":"main","identity":{"value":"marie@box","source":"fallback"},"claim":null,"drift":null,"also_held":[],"remote":false,"refs":null,"elsewhere":[],"constraints":0,"queue":0,"unmerged":0,"faults":0,"signals":0}"#;

const CONTEXT: &str = r#"{"contract":1,"mode":"orientation","head":null,"criteria":null,"method":null,"constraints":[],"proposed":[],"specs":[],"tasks":[
 {"id":"TASK-c81aba215f4d","short":"TASK-c81a","title":"Work a task","status":"open","ready":true,"unblocks":1,"state":"open"},
 {"id":"TASK-416cdde27bfb","short":"TASK-416c","title":"Sync engine","status":"in_progress","ready":true,"unblocks":1,"state":"claimed:ank-2"},
 {"id":"TASK-91a1aaaaaaaa","short":"TASK-91a1","title":"Daemon","status":"open","ready":false,"unblocks":2,"state":"open"}
],"log":[],"ready":2,"blocked":1,"finished_elsewhere":0,"warnings":[]}"#;

fn find_claimed_by(holder: &str) -> String {
    format!(
        r#"{{"contract":1,"corpus":null,"total":1,"shown":1,"hidden":0,"results":[{{"id":"TASK-c81aba215f4d","kind":"task","status":"in_progress","state":"claimed:{holder}","title":"Work a task","created":"2026-09-25T16:54:28Z","archived":false}}]}}"#
    )
}

struct Setup {
    dir: PathBuf,
    corpus: PathBuf,
    work: Work,
}

/// A workspace whose cwd is a subdirectory of a corpus at `<dir>/repo`.
fn setup(name: &str, pick: &str, status: &str, find: &str) -> Setup {
    let dir = scratch(name);
    let corpus = dir.join("repo");
    fs::create_dir_all(corpus.join(".ank")).unwrap();
    fs::create_dir_all(corpus.join("src/deep")).unwrap();
    support::write_answers(
        &dir,
        &[
            support::answer(&["status"], status, "", 0),
            support::answer(&["context"], CONTEXT, "", 0),
            support::answer(&["find"], find, "", 0),
        ],
    );
    let state = dir.join("state");
    fs::create_dir_all(&state).unwrap();
    let herdr = fake_herdr(&dir, &state, pick);
    let context = serde_json::json!({
        "workspace_id": "w1",
        "workspace_cwd": corpus.join("src/deep"),
        "focused_pane_id": "w1:p1",
        "invocation_source": "command_palette",
        "a_field_herdr_added_later": 1,
    })
    .to_string();
    let work = Work {
        herdr,
        ank_program: fake_ank().to_path_buf(),
        context_json: context,
        state_dir: state,
        worktrees_root: dir.join("worktrees"),
        config: Config::default(),
        poll: Duration::from_millis(10),
        pick_timeout: Duration::from_secs(5),
        claim_timeout: Duration::from_millis(500),
    };
    Setup { dir, corpus, work }
}

#[test]
fn work_chains_the_herdr_calls_in_order_after_the_pick() {
    let s = setup(
        "chain",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("marie@box/ank-c81a"),
    );
    let outcome = work::run(&s.work).unwrap();
    let worktree = s.dir.join("worktrees/repo/ank-c81a");
    assert_eq!(
        outcome,
        Outcome::Claimed {
            id: "TASK-c81aba215f4d".into(),
            agent: "ank-c81a".into(),
            worktree: worktree.clone(),
        }
    );
    let wt = worktree.display();
    assert_eq!(
        calls(&s.dir),
        [
            "plugin|pane|open|--plugin|ank|--entrypoint|pick|--workspace|w1|--env|HERDR_ANK_REPO={CORPUS}|--focus"
                .replace("{CORPUS}", s.corpus.to_str().unwrap()),
            format!("worktree|create|--workspace|w1|--branch|task/c81a|--base|main|--path|{wt}|--label|ank-c81a|--no-focus"),
            format!("tab|create|--workspace|w1|--cwd|{wt}|--env|ANK_AGENT=marie@box/ank-c81a|--label|ank-c81a|--focus"),
            "agent|start|ank-c81a|--kind|claude|--pane|w1:p-tab".to_owned(),
            "agent|prompt|ank-c81a|ank claim TASK-c81aba215f4d".to_owned(),
        ]
    );
}

#[test]
fn the_agent_kind_comes_from_the_config() {
    let mut s = setup(
        "kind",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("marie@box/ank-c81a"),
    );
    s.work.config.agent.kind = "codex".into();
    work::run(&s.work).unwrap();
    assert!(calls(&s.dir).contains(&"agent|start|ank-c81a|--kind|codex|--pane|w1:p-tab".into()));
}

#[test]
fn the_agent_args_come_from_the_config() {
    let mut s = setup(
        "args",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("marie@box/ank-c81a"),
    );
    s.work.config.agent.args = vec!["--model".into(), "opus".into()];
    work::run(&s.work).unwrap();
    assert!(calls(&s.dir)
        .contains(&"agent|start|ank-c81a|--kind|claude|--pane|w1:p-tab|--|--model|opus".into()));
}

#[test]
fn a_null_default_branch_starts_the_worktree_from_head() {
    let status = STATUS.replace(r#""default_branch":"main""#, r#""default_branch":null"#);
    let s = setup(
        "head",
        "TASK-c81aba215f4d",
        &status,
        &find_claimed_by("marie@box/ank-c81a"),
    );
    work::run(&s.work).unwrap();
    assert!(calls(&s.dir)[1].contains("|--base|HEAD|"));
}

#[test]
fn a_claim_held_elsewhere_notifies_and_keeps_the_worktree() {
    let s = setup(
        "taken",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("jo@other/ank-7"),
    );
    let outcome = work::run(&s.work).unwrap();
    assert_eq!(
        outcome,
        Outcome::Unavailable {
            id: "TASK-c81aba215f4d".into(),
            holder: Some("jo@other/ank-7".into()),
            worktree: s.dir.join("worktrees/repo/ank-c81a"),
        }
    );
    let calls = calls(&s.dir);
    assert_eq!(calls.len(), 6, "{calls:#?}");
    assert!(calls[4].starts_with("agent|prompt|"));
    assert!(calls[5].starts_with("notification|show|"), "{}", calls[5]);
    assert!(calls[5].contains("TASK-c81a"));
    assert!(calls[5].contains("jo@other/ank-7"));
    assert!(!calls.iter().any(|c| c.starts_with("worktree|remove")));
}

#[test]
fn a_task_finished_elsewhere_is_unavailable_too() {
    let find = find_claimed_by("x").replace(
        r#""status":"in_progress","state":"claimed:x""#,
        r#""status":"done","state":"done""#,
    );
    let s = setup("finished", "TASK-c81aba215f4d", STATUS, &find);
    assert!(matches!(
        work::run(&s.work).unwrap(),
        Outcome::Unavailable { holder: None, .. }
    ));
    assert!(calls(&s.dir)[5].starts_with("notification|show|"));
}

#[test]
fn a_cancelled_pick_calls_nothing_after_the_pane() {
    let s = setup("cancel", "", STATUS, &find_claimed_by("x"));
    assert_eq!(work::run(&s.work).unwrap(), Outcome::Cancelled);
    assert_eq!(calls(&s.dir).len(), 1);
}

#[test]
fn a_workspace_without_a_corpus_is_refused_before_any_herdr_call() {
    let s = setup(
        "nocorpus",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("x"),
    );
    let mut work = s.work;
    // Outside the target directory: this repository is itself a corpus.
    let bare = std::env::temp_dir().join(format!("herdr-ank-no-corpus-{}/sub", std::process::id()));
    assert_eq!(
        work::find_corpus(&bare.join("..")),
        None,
        "a .ank/ above the temp dir"
    );
    fs::create_dir_all(&bare).unwrap();
    work.context_json =
        serde_json::json!({"workspace_id": "w1", "workspace_cwd": bare}).to_string();
    match work::run(&work) {
        Err(err @ WorkError::NoCorpus { .. }) => {
            let message = err.to_string();
            assert!(message.contains(bare.to_str().unwrap()), "{message}");
            assert!(message.contains("ank init"), "{message}");
        }
        other => panic!("expected NoCorpus, got {other:?}"),
    }
    assert!(calls(&s.dir).is_empty());
}

#[test]
fn a_context_without_workspace_is_refused() {
    let s = setup(
        "noworkspace",
        "TASK-c81aba215f4d",
        STATUS,
        &find_claimed_by("x"),
    );
    let mut work = s.work;
    work.context_json = "{}".into();
    assert!(matches!(work::run(&work), Err(WorkError::NoWorkspace)));
    assert!(calls(&s.dir).is_empty());
}

fn task(id: &str, title: &str, ready: bool, state: &str) -> ContextTask {
    serde_json::from_value(serde_json::json!({
        "id": id, "short": &id[..9], "title": title, "status": "open",
        "ready": ready, "unblocks": 0, "state": state
    }))
    .unwrap()
}

#[test]
fn the_picker_lists_only_tasks_neither_held_nor_blocked() {
    let tasks = vec![
        task("TASK-aaaa0001", "Free", true, "open"),
        task("TASK-bbbb0002", "Held", true, "claimed:ank-2"),
        task("TASK-cccc0003", "Blocked", false, "open"),
    ];
    let listed: Vec<_> = pick::claimable(tasks).into_iter().map(|t| t.id).collect();
    assert_eq!(listed, ["TASK-aaaa0001"]);
}

#[test]
fn the_picker_filter_matches_id_or_title_ignoring_case() {
    let tasks = vec![
        task("TASK-aaaa0001", "Sync engine", true, "open"),
        task("TASK-bbbb0002", "Notifications", true, "open"),
    ];
    let ids = |filter: &str| -> Vec<String> {
        pick::filter(&tasks, filter)
            .into_iter()
            .map(|t| t.id.clone())
            .collect()
    };
    assert_eq!(ids(""), ["TASK-aaaa0001", "TASK-bbbb0002"]);
    assert_eq!(ids("NOTIF"), ["TASK-bbbb0002"]);
    assert_eq!(ids("aaaa"), ["TASK-aaaa0001"]);
    assert!(ids("zzz").is_empty());
}

#[test]
fn the_picker_keys_filter_move_and_select() {
    use pick::{Key, Picker, Step};
    let mut picker = Picker::new(vec![
        task("TASK-aaaa0001", "Sync engine", true, "open"),
        task("TASK-bbbb0002", "Notifications", true, "open"),
        task("TASK-cccc0003", "Notes", true, "open"),
    ]);
    assert_eq!(picker.key(Key::Down), Step::Continue);
    assert_eq!(picker.key(Key::Char('n')), Step::Continue);
    assert_eq!(picker.key(Key::Char('o')), Step::Continue);
    // Two rows match "no"; the cursor is clamped back onto them.
    assert_eq!(picker.visible().len(), 2);
    assert_eq!(picker.key(Key::Down), Step::Continue);
    assert_eq!(
        picker.key(Key::Enter),
        Step::Selected("TASK-cccc0003".into())
    );
    assert_eq!(picker.key(Key::Backspace), Step::Continue);
    assert_eq!(picker.key(Key::Esc), Step::Cancelled);
}

#[test]
fn the_picker_writes_its_selection_into_the_state_dir() {
    let dir = scratch("selection");
    pick::write_selection(&dir, Some("TASK-aaaa0001")).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("pick")).unwrap(),
        "TASK-aaaa0001"
    );
    pick::write_selection(&dir, None).unwrap();
    assert_eq!(fs::read_to_string(dir.join("pick")).unwrap(), "");
}

#[test]
fn the_manifest_declares_the_work_action_and_the_pick_popup() {
    let manifest: toml::Table =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/herdr-plugin.toml"))
            .unwrap()
            .parse()
            .unwrap();
    let find = |table: &str, id: &str| -> toml::Table {
        manifest[table]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_table().unwrap().clone())
            .find(|e| e["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("no [[{table}]] id = {id:?}"))
    };
    let action = find("actions", "work");
    assert_eq!(
        action["contexts"].as_array().unwrap(),
        &[toml::Value::from("workspace")]
    );
    let command: Vec<_> = action["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        command[0].ends_with("herdr-ank") && command[1..] == ["work"],
        "{command:?}"
    );

    let pane = find("panes", "pick");
    assert_eq!(pane["placement"].as_str(), Some("popup"));
    let command: Vec<_> = pane["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        command[0].ends_with("herdr-ank") && command[1..] == ["pick"],
        "{command:?}"
    );
}
