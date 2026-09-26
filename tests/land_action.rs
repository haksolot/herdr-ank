//! The "Land a task" action, end to end: a fake `herdr` that records every
//! call and plays the picker, a fake `ank` that answers from documents the
//! test writes, and real git repositories in a temporary directory, an
//! integration tree on `main`, the task's worktree on `task/abcd` and a bare
//! `origin`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use herdr_ank::ank;
use herdr_ank::config::Config;
use herdr_ank::herdr;
use herdr_ank::land::action::{self, Land, Outcome};
use serde_json::{json, Value};

mod support;

const ID: &str = "TASK-abcd00000000";
const BRANCH: &str = "task/abcd";

fn scratch(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "land-action-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The fake `ank`, shared: its home is the parent of the `--repo <dir>/repo`
/// it is handed.
fn fake_ank() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = scratch("ank-bin");
        support::home_from_last_arg(&dir);
        support::link(&dir, "ank")
    })
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn git_ok(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap()
        .status
        .success()
}

fn commit(dir: &Path, file: &str, message: &str) {
    if let Some(parent) = Path::new(file).parent() {
        fs::create_dir_all(dir.join(parent)).unwrap();
    }
    fs::write(dir.join(file), format!("{message}\n")).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", message]);
}

struct Setup {
    dir: PathBuf,
    repo: PathBuf,
    worktree: PathBuf,
    origin: PathBuf,
    land: Land,
}

fn str_of(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// `<dir>/repo` on `main`, a corpus; `task/abcd` in its own worktree at
/// `<dir>/wt/ank-abcd`, one commit ahead of the base and pushed to the bare
/// `<dir>/origin.git`; `main` moved once since.
fn repos(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let repo = dir.join("repo");
    let worktree = dir.join("wt").join("ank-abcd");
    let origin = dir.join("origin.git");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.name", "land test"),
        ("user.email", "land@test"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
    ] {
        git(&repo, &["config", key, value]);
    }
    commit(&repo, ".ank/keep", "base");
    git(dir, &["init", "-q", "--bare", str_of(&origin)]);
    git(&repo, &["remote", "add", "origin", str_of(&origin)]);
    git(
        &repo,
        &["worktree", "add", "-q", "-b", BRANCH, str_of(&worktree)],
    );
    commit(&worktree, "task.txt", "task work");
    git(&worktree, &["push", "-q", "origin", BRANCH]);
    commit(&repo, "main.txt", "main moves");
    (repo, worktree, origin)
}

const STATUS: &str = r#"{"contract":1,"corpus":null,"branch":"main","default_branch":"main","identity":{"value":"marie@box","source":"fallback"},"claim":null,"drift":null,"also_held":[],"remote":false,"refs":null,"elsewhere":[],"constraints":0,"queue":0,"unmerged":0,"faults":0,"signals":0}"#;

fn row(id: &str, status: &str, title: &str) -> Value {
    json!({"id": id, "kind": "task", "status": status, "state": status, "title": title,
           "created": "2026-09-26T00:00:00Z", "archived": false})
}

/// A task finished on its branch, as the integration tree's corpus sees it
/// until it lands: still open there, with the proof on the branch.
fn finished(id: &str, title: &str) -> Value {
    let mut row = row(id, "open", title);
    row["state"] = json!(format!("finished:1234567 on {BRANCH}"));
    row
}

fn find(rows: &[Value]) -> String {
    json!({"contract": 1, "corpus": null, "total": rows.len(), "shown": rows.len(),
           "hidden": 0, "results": rows})
    .to_string()
}

fn ok(args: &[&str]) -> Value {
    support::answer(args, r#"{"id":"cli","result":{"type":"ok"}}"#, "", 0)
}

/// The panes herdr lists: one of the repository's tabs sits in the worktree
/// (`w1:t4`), another in the repository (`w1:t2`), and the worktree's own
/// workspace `w9`, opened by `worktree create`, has its tab `w9:t1`.
fn panes(repo: &Path, worktree: &Path) -> String {
    let pane = |id: &str, tab: &str, cwd: &Path| {
        json!({"pane_id": id, "workspace_id": &tab[..2], "tab_id": tab,
               "agent_status": "unknown", "cwd": cwd, "focused": false, "revision": 1})
    };
    json!({"id": "cli:pane:list", "result": {"type": "pane_list", "panes": [
        pane("w1:p1", "w1:t2", repo),
        pane("w1:p7", "w1:t4", &worktree.join("src")),
        pane("w1:p8", "w1:t4", worktree),
        pane("w9:p1", "w9:t1", worktree),
    ]}})
    .to_string()
}

fn worktrees(repo: &Path, worktree: &Path) -> String {
    json!({"id": "cli:worktree:list", "result": {"type": "worktree_list",
    "source": {"repo_root": repo, "source_workspace_id": "w1"},
    "worktrees": [
        {"branch": "main", "path": repo, "open_workspace_id": "w1", "is_linked_worktree": false},
        {"branch": BRANCH, "path": worktree, "open_workspace_id": "w9", "is_linked_worktree": true},
    ]}})
    .to_string()
}

fn herdr_home(dir: &Path) -> PathBuf {
    dir.join("herdr-home")
}

/// A fake `herdr` answering as 0.9.1 does. `plugin pane open` plays the
/// picker by copying `pick` into the state directory; `worktree remove`
/// removes the checkout as herdr does. `extra` answers come first.
fn fake_herdr(s: &Paths, pick: &str, extra: &[Value]) -> herdr::Client {
    let home = herdr_home(&s.dir);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("pick"), pick).unwrap();
    let bin = support::link(&home, "herdr");
    let mut picked = support::answer(
        &["plugin", "pane"],
        // As herdr 0.9.1 answers a popup: no pane is described.
        r#"{"id":"cli:plugin","result":{"type":"ok"}}"#,
        "",
        0,
    );
    picked["copy"] = json!([home.join("pick"), s.state.join("pick")]);
    let mut remove = support::answer(
        &["worktree", "remove"],
        r#"{"id":"cli:worktree:remove","result":{"type":"worktree_removed","forced":false}}"#,
        "",
        0,
    );
    remove["run"] = json!(["git", "-C", s.repo, "worktree", "remove", "--force", s.worktree]);
    let mut answers = extra.to_vec();
    answers.extend([
        picked,
        support::answer(
            &["worktree", "list"],
            &worktrees(&s.repo, &s.worktree),
            "",
            0,
        ),
        remove,
        support::answer(&["pane", "list"], &panes(&s.repo, &s.worktree), "", 0),
        ok(&[]),
    ]);
    support::write_answers(&home, &answers);
    herdr::Client::new(bin, s.dir.join("herdr.sock"))
}

struct Paths {
    dir: PathBuf,
    repo: PathBuf,
    worktree: PathBuf,
    state: PathBuf,
}

/// The whole scene; `status` is the task's status as ank reports it.
fn setup_with(name: &str, pick: &str, status: &str, extra_herdr: &[Value]) -> Setup {
    let dir = scratch(name);
    let (repo, worktree, origin) = repos(&dir);
    let state = dir.join("state");
    fs::create_dir_all(&state).unwrap();
    // The integration tree's corpus sees the task finished elsewhere; the
    // fake answering `--repo <dir>/wt/ank-abcd` has its home in `<dir>/wt`,
    // and answers with the task's `status` as its own branch holds it.
    support::write_answers(
        &dir,
        &[
            support::answer(&["status"], STATUS, "", 0),
            support::answer(&["find"], &find(&[finished(ID, "a task")]), "", 0),
        ],
    );
    support::write_answers(
        &dir.join("wt"),
        &[
            support::answer(&["status"], STATUS, "", 0),
            support::answer(&["find"], &find(&[row(ID, status, "a task")]), "", 0),
        ],
    );
    let paths = Paths {
        dir: dir.clone(),
        repo: repo.clone(),
        worktree: worktree.clone(),
        state: state.clone(),
    };
    let herdr = fake_herdr(&paths, pick, extra_herdr);
    let context = json!({
        "workspace_id": "w1",
        "workspace_cwd": repo,
        "focused_pane_id": "w1:p1",
        "a_field_herdr_added_later": 1,
    })
    .to_string();
    let land = Land {
        herdr,
        ank_program: fake_ank().to_path_buf(),
        context_json: context,
        state_dir: state,
        config: Config::default().land,
        poll: Duration::from_millis(10),
        pick_timeout: Duration::from_secs(5),
    };
    Setup {
        dir,
        repo,
        worktree,
        origin,
        land,
    }
}

fn setup(name: &str) -> Setup {
    setup_with(name, ID, "done", &[])
}

/// Each herdr call, its arguments joined by `|`.
fn calls(s: &Setup) -> Vec<String> {
    support::calls(&herdr_home(&s.dir))
        .into_iter()
        .map(|call| call.argv.join("|"))
        .collect()
}

fn notifications(s: &Setup) -> Vec<String> {
    calls(s)
        .into_iter()
        .filter(|c| c.starts_with("notification|show|"))
        .collect()
}

fn local_branch(s: &Setup) -> bool {
    git_ok(
        &s.repo,
        &["rev-parse", "--verify", "--quiet", "refs/heads/task/abcd"],
    )
}

fn remote_branch(s: &Setup) -> bool {
    !git(&s.origin, &["branch", "--list", BRANCH]).is_empty()
}

fn pick_open(s: &Setup) -> String {
    format!(
        "plugin|pane|open|--plugin|ank|--entrypoint|pick|--env|HERDR_ANK_REPO={}|--env|HERDR_ANK_PICK=land|--focus",
        s.repo.display()
    )
}

const LANDED: &str = "notification|show|TASK-abcd landée sur main";

#[test]
fn landing_then_cleans_up_in_order_tabs_worktree_branch_remote() {
    let s = setup("order");
    let task_tip = git(&s.worktree, &["rev-parse", "HEAD"]);
    let outcome = action::run(&s.land).unwrap();
    let head = git(&s.repo, &["rev-parse", "main"]);
    assert_eq!(
        outcome,
        Outcome::Landed {
            id: ID.into(),
            head: head.clone(),
            failures: vec![],
        }
    );
    assert_ne!(head, task_tip, "the task branch was rebased on main first");
    assert_eq!(
        git(&s.repo, &["show", "-s", "--format=%s", "main"]),
        "task work"
    );
    assert_eq!(
        calls(&s),
        [
            pick_open(&s),
            "worktree|list|--workspace|w1".to_owned(),
            "pane|list".to_owned(),
            "tab|close|w1:t4".to_owned(),
            "worktree|remove|--workspace|w9".to_owned(),
            format!("{LANDED}|--sound|done"),
        ]
    );
    assert!(!s.worktree.exists(), "the worktree was removed");
    assert!(!local_branch(&s), "the local branch was deleted");
    assert!(!remote_branch(&s), "the branch was deleted on origin");
}

#[test]
fn close_tab_false_leaves_every_tab() {
    let mut s = setup("no-tab");
    s.land.config.close_tab = false;
    action::run(&s.land).unwrap();
    let calls = calls(&s);
    assert!(
        !calls.iter().any(|c| c.starts_with("pane|list")),
        "{calls:#?}"
    );
    assert!(
        !calls.iter().any(|c| c.starts_with("tab|close")),
        "{calls:#?}"
    );
    assert!(calls.contains(&"worktree|remove|--workspace|w9".to_owned()));
    assert!(!local_branch(&s));
    assert!(!remote_branch(&s));
}

#[test]
fn remove_worktree_false_keeps_it_and_closes_its_workspace_tab_too() {
    let mut s = setup("no-worktree");
    s.land.config.remove_worktree = false;
    let outcome = action::run(&s.land).unwrap();
    let calls = calls(&s);
    assert!(
        !calls.iter().any(|c| c.starts_with("worktree|remove")),
        "{calls:#?}"
    );
    assert!(calls.contains(&"tab|close|w1:t4".to_owned()));
    assert!(calls.contains(&"tab|close|w9:t1".to_owned()));
    assert!(s.worktree.exists());
    // git refuses to delete a branch a worktree still has checked out: a
    // cleanup failure, notified, and the remote step still runs.
    assert!(local_branch(&s));
    let Outcome::Landed { failures, .. } = outcome else {
        panic!("expected Landed, got {outcome:?}");
    };
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(!remote_branch(&s));
    assert!(notifications(&s).last().unwrap().starts_with(LANDED));
}

#[test]
fn delete_branch_false_keeps_the_local_branch() {
    let mut s = setup("no-branch");
    s.land.config.delete_branch = false;
    action::run(&s.land).unwrap();
    assert!(!s.worktree.exists());
    assert!(local_branch(&s));
    assert!(!remote_branch(&s));
}

#[test]
fn delete_remote_branch_false_keeps_it_on_origin() {
    let mut s = setup("no-remote");
    s.land.config.delete_remote_branch = false;
    action::run(&s.land).unwrap();
    assert!(!local_branch(&s));
    assert!(remote_branch(&s));
}

#[test]
fn a_branch_absent_from_origin_is_not_pushed_and_is_no_failure() {
    let s = setup("absent-remote");
    git(&s.origin, &["branch", "-D", BRANCH]);
    let outcome = action::run(&s.land).unwrap();
    assert!(
        matches!(&outcome, Outcome::Landed { failures, .. } if failures.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(notifications(&s), [format!("{LANDED}|--sound|done")]);
}

#[test]
fn a_refusal_notifies_its_cause_and_remedy_and_cleans_nothing() {
    let s = setup_with("refused", ID, "in_progress", &[]);
    let main_before = git(&s.repo, &["rev-parse", "main"]);
    let outcome = action::run(&s.land).unwrap();
    assert!(
        matches!(&outcome, Outcome::Refused { id, .. } if id == ID),
        "{outcome:?}"
    );
    let calls = calls(&s);
    assert_eq!(calls.len(), 3, "{calls:#?}");
    assert_eq!(calls[1], "worktree|list|--workspace|w1");
    let notice = &calls[2];
    assert!(notice.starts_with("notification|show|"), "{notice}");
    assert!(notice.contains("TASK-abcd"), "{notice}");
    assert!(notice.contains("in_progress"), "{notice}");
    assert!(notice.contains("ank done"), "{notice}");
    assert_eq!(git(&s.repo, &["rev-parse", "main"]), main_before);
    assert!(s.worktree.exists());
    assert!(local_branch(&s));
    assert!(remote_branch(&s));
}

#[test]
fn a_dirty_worktree_is_refused_with_the_command_that_lifts_it() {
    let s = setup("dirty");
    fs::write(s.worktree.join("wip.txt"), "wip\n").unwrap();
    let outcome = action::run(&s.land).unwrap();
    assert!(matches!(outcome, Outcome::Refused { .. }), "{outcome:?}");
    let notices = notifications(&s);
    assert_eq!(notices.len(), 1, "{notices:#?}");
    assert!(notices[0].contains("git -C"), "{}", notices[0]);
    assert!(notices[0].contains("status"), "{}", notices[0]);
    assert!(s.worktree.exists());
    assert!(local_branch(&s));
}

#[test]
fn a_cleanup_failure_is_notified_and_the_next_steps_still_run() {
    let s = setup_with(
        "tab-fails",
        ID,
        "done",
        &[support::answer(
            &["tab", "close"],
            "",
            r#"{"error":{"code":"tab_not_found","message":"tab w1:t4 not found"},"id":"cli:tab:close"}"#,
            1,
        )],
    );
    let outcome = action::run(&s.land).unwrap();
    let Outcome::Landed { failures, .. } = outcome else {
        panic!("expected Landed, got {outcome:?}");
    };
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("w1:t4"), "{failures:?}");
    let notices = notifications(&s);
    assert_eq!(notices.len(), 2, "{notices:#?}");
    assert!(notices[0].contains("w1:t4"), "{}", notices[0]);
    assert!(notices[1].starts_with(LANDED), "{}", notices[1]);
    assert!(calls(&s).contains(&"worktree|remove|--workspace|w9".to_owned()));
    assert!(!local_branch(&s));
    assert!(!remote_branch(&s));
}

#[test]
fn a_cancelled_pick_lands_nothing() {
    let s = setup_with("cancel", "", "done", &[]);
    assert_eq!(action::run(&s.land).unwrap(), Outcome::Cancelled);
    assert_eq!(calls(&s), [pick_open(&s)]);
    assert!(local_branch(&s));
}

#[test]
fn the_land_picker_lists_only_done_tasks_with_an_unlanded_branch() {
    let s = setup("landable");
    // TASK-abcd is finished on its branch, which the integration tree's
    // corpus reads as open; TASK-5555 is done there, its branch not in main
    // yet. task/ef01 is done and already in main; task/9999 has no branch;
    // the open task has one but is not done.
    git(&s.repo, &["branch", "task/ef01", "main"]);
    git(&s.repo, &["branch", "task/7777", BRANCH]);
    git(&s.repo, &["branch", "task/5555", BRANCH]);
    support::write_answers(
        &s.dir,
        &[
            support::answer(&["status"], STATUS, "", 0),
            support::answer(
                &["find"],
                &find(&[
                    finished(ID, "Land me"),
                    row("TASK-555500000000", "done", "Land me too"),
                    row("TASK-ef0100000000", "done", "Landed already"),
                    row("TASK-999900000000", "done", "No branch"),
                    row("TASK-777700000000", "open", "Not done"),
                ]),
                "",
                0,
            ),
        ],
    );
    let client = ank::Client::with_program(fake_ank(), &s.repo);
    let listed = action::landable_tasks(&client).unwrap();
    let rows: Vec<_> = listed
        .iter()
        .map(|t| (t.id.as_str(), t.short.as_str(), t.title.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (ID, "TASK-abcd", "Land me"),
            ("TASK-555500000000", "TASK-5555", "Land me too")
        ]
    );
    let ank_calls = support::calls(&s.dir);
    let find = ank_calls
        .iter()
        .find(|c| c.argv[0] == "find")
        .expect("ank find was run");
    assert_eq!(find.argv[1..4], ["--type", "task", "--json"]);
}

#[test]
fn the_manifest_declares_the_land_action() {
    let manifest: toml::Table =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/herdr-plugin.toml"))
            .unwrap()
            .parse()
            .unwrap();
    let action = manifest["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_table().unwrap())
        .find(|e| e["id"].as_str() == Some("land"))
        .expect("no [[actions]] id = \"land\"");
    assert_eq!(action["title"].as_str(), Some("Land a task"));
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
    assert_eq!(command, ["./bin/herdr-ank", "land"]);
}
