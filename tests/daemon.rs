//! The daemon's single instance and its coalescing (TASK-91a1, ADR-6fb76f3a1197).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use herdr_ank::daemon::{minutes_until, Lock, Schedule};

mod support;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn tempdir(name: &str) -> PathBuf {
    let n = NEXT_DIR.fetch_add(1, Ordering::SeqCst);
    // Under the target dir, where the fake is hard-linked rather than
    // copied: a tmpfs /tmp fills up with one copy per test.
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "herdr-ank-daemon-{}-{name}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const SECOND: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_secs(30);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn a_second_lock_on_the_state_dir_is_refused_until_the_first_is_dropped() {
    let dir = tempdir("lock");
    let first = Lock::acquire(&dir).unwrap().expect("a free lock is taken");
    assert!(Lock::acquire(&dir).unwrap().is_none(), "the lock is held");
    drop(first);
    assert!(
        Lock::acquire(&dir).unwrap().is_some(),
        "a dropped lock is free"
    );
}

#[test]
fn a_dropped_lock_is_free_while_another_test_spawns_processes() {
    let dir = tempdir("lock-vs-spawn");
    let stop = Arc::new(AtomicBool::new(false));
    let spawner = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let mut command = Command::new(env!("CARGO_BIN_EXE_herdr-ank"));
                command.stdout(Stdio::null()).stderr(Stdio::null());
                let _ = command.spawn().unwrap().wait();
            }
        })
    };
    let rounds = 2000;
    let mut refused = 0;
    for _ in 0..rounds {
        // Each round starts and ends with the lock free: any refusal is a lock
        // some other holder kept after this test dropped it.
        for _ in 0..2 {
            match Lock::acquire(&dir).unwrap() {
                Some(lock) => drop(lock),
                None => refused += 1,
            }
        }
    }
    stop.store(true, Ordering::SeqCst);
    spawner.join().unwrap();
    assert_eq!(
        refused, 0,
        "the lock was held after its drop {refused}/{rounds} times"
    );
}

#[test]
fn the_daemon_exits_zero_at_once_when_another_instance_holds_the_lock() {
    let dir = tempdir("second-instance");
    let _held = Lock::acquire(&dir).unwrap().unwrap();

    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_herdr-ank"))
        .arg("daemon")
        .env("HERDR_PLUGIN_STATE_DIR", &dir)
        .env_remove("HERDR_SOCKET_PATH")
        .env_remove("HERDR_BIN_PATH")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            panic!("the second daemon did not exit while the lock was held");
        }
        std::thread::sleep(ms(20));
    };
    assert!(status.success(), "{status}");
}

#[test]
fn the_first_sync_runs_at_startup() {
    let t0 = Instant::now();
    let schedule = Schedule::new(SECOND, POLL);
    assert!(schedule.due(t0));
}

#[test]
fn a_burst_of_events_within_one_second_is_one_sync() {
    let t0 = Instant::now();
    let mut schedule = Schedule::new(SECOND, POLL);
    schedule.ran(t0);

    let mut syncs = 0;
    for offset in [100, 150, 400, 700, 950, 999] {
        let now = t0 + ms(offset);
        schedule.trigger();
        if schedule.due(now) {
            syncs += 1;
            schedule.ran(now);
        }
    }
    assert_eq!(syncs, 0, "nothing runs before a second has passed");
    assert_eq!(schedule.wait(t0 + ms(999)), ms(1));
    assert!(schedule.due(t0 + SECOND));
    schedule.ran(t0 + SECOND);
    assert!(
        !schedule.due(t0 + SECOND + ms(500)),
        "the burst was spent in one sync"
    );
}

#[test]
fn an_event_long_after_the_last_sync_runs_at_once() {
    let t0 = Instant::now();
    let mut schedule = Schedule::new(SECOND, POLL);
    schedule.ran(t0);
    schedule.trigger();
    assert!(schedule.due(t0 + ms(5_000)));
    assert_eq!(schedule.wait(t0 + ms(5_000)), Duration::ZERO);
}

#[test]
fn without_events_a_sync_still_runs_every_poll_interval() {
    let t0 = Instant::now();
    let mut schedule = Schedule::new(SECOND, POLL);
    schedule.ran(t0);
    assert!(!schedule.due(t0 + ms(29_999)));
    assert_eq!(schedule.wait(t0 + ms(20_000)), ms(10_000));
    assert!(schedule.due(t0 + POLL));
}

#[test]
fn a_claim_expiry_reads_as_whole_minutes_left_rounded_up() {
    // 2026-09-25T18:34:14Z
    let expiry = "2026-09-25T18:34:14Z";
    let unix = 1_790_361_254;
    assert_eq!(minutes_until(expiry, unix - 30 * 60), Some(30));
    assert_eq!(minutes_until(expiry, unix - 29 * 60 - 1), Some(30));
    assert_eq!(minutes_until(expiry, unix + 5), Some(0));
    assert_eq!(minutes_until("not a date", unix), None);
}

/// ank-2's measurement on the daemon of TASK-91a1: the bare `<user>@<host>`
/// holds a claim while tasks are claimable. Read as `…/herdr-ank`, `context`
/// stays in orientation mode and says 3; the pane shows the claim and the 3.
#[test]
fn a_bare_identity_claim_leaves_the_queue_counted() {
    use herdr_ank::config::Config;
    use herdr_ank::herdr::Pane;
    use herdr_ank::sync::{plan, Corpus};

    let pane: Pane = serde_json::from_value(serde_json::json!({
        "pane_id": "w1:p2", "workspace_id": "w1", "tab_id": "w1:t1",
        "cwd": "/src/repo", "name": "ank-2",
    }))
    .unwrap();
    let corpus = Corpus {
        root: PathBuf::from("/src/repo"),
        in_progress: serde_json::from_value(serde_json::json!({
            "contract": 1, "corpus": null, "total": 1, "shown": 1, "hidden": 0,
            "results": [{
                "id": "TASK-416cdde27bfb", "kind": "task", "status": "in_progress",
                "state": "claimed:marie@box", "title": "Sync engine",
                "created": "2026-09-25T16:54:25Z", "archived": false,
            }],
        }))
        .unwrap(),
        context: serde_json::from_value(serde_json::json!({
            "contract": 1, "mode": "orientation", "head": null, "criteria": null,
            "method": null, "constraints": [], "proposed": [], "specs": [], "tasks": [],
            "log": [], "ready": 3, "blocked": 0, "finished_elsewhere": 0, "warnings": [],
        }))
        .unwrap(),
        expires_in: Default::default(),
    };
    let reports = plan(
        std::slice::from_ref(&pane),
        std::slice::from_ref(&pane),
        &[corpus],
        &Config::default(),
    );
    assert_eq!(reports.len(), 1);
    let tokens: std::collections::BTreeMap<_, _> = reports[0]
        .tokens
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert_eq!(tokens.get("ank_task"), Some(&"TASK-416c"));
    assert_eq!(tokens.get("ank_queue"), Some(&"3"));
}

/// Runs `herdr-ank daemon` as an `[[events]]` hook would (ADR-6fb76f3a1197).
fn daemon_from_event(state_dir: &std::path::Path, event: &str) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_herdr-ank"))
        .arg("daemon")
        .env("HERDR_PLUGIN_EVENT", event)
        .env("HERDR_PLUGIN_STATE_DIR", state_dir)
        .env("HERDR_PLUGIN_CONFIG_DIR", state_dir)
        .env("HERDR_BIN_PATH", "/nonexistent/herdr")
        .env("HERDR_SOCKET_PATH", state_dir.join("absent.sock"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn an_event_hook_finding_the_lock_held_exits_zero_and_says_nothing() {
    let dir = tempdir("event-held");
    let _held = Lock::acquire(&dir).unwrap().unwrap();

    let mut child = daemon_from_event(&dir, "tab.created");
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            panic!("the hook did not exit while the lock was held");
        }
        std::thread::sleep(ms(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn an_event_hook_finding_the_lock_free_becomes_the_daemon() {
    let dir = tempdir("event-free");

    let mut child = daemon_from_event(&dir, "tab.created");
    let started = Instant::now();
    let held = loop {
        if Lock::acquire(&dir).unwrap().is_none() {
            break true;
        }
        if child.try_wait().unwrap().is_some() || started.elapsed() > Duration::from_secs(5) {
            break false;
        }
        std::thread::sleep(ms(20));
    };
    let alive = child.try_wait().unwrap().is_none();
    let _ = child.kill();
    let _ = child.wait();
    assert!(held, "the hook never took the lock");
    assert!(alive, "the hook exited instead of staying as the daemon");
}

/// Runs one `herdr-ank sync` pass against a fake herdr and a fake ank
/// sharing `bin/`, with a corpus at `<dir>/repo` whose find lists one claim
/// by `marie@box/ank-2`, whose context says 3 claimable and whose status
/// says 1 decision queued. `panes` builds `pane list` from the repo path;
/// `agent list` holds those with an `agent` field, named `ank-2`.
fn sync_pass(
    name: &str,
    panes: impl Fn(&std::path::Path) -> Vec<serde_json::Value>,
) -> Vec<support::Call> {
    sync_pass_with(name, panes, AnkAt::OnPath)
}

/// Where the fake ank lives, as herdr's environment may leave it.
#[derive(Clone, Copy, PartialEq)]
enum AnkAt {
    /// Next to the fake herdr, which is `PATH`.
    OnPath,
    /// Next to the fake herdr, and `PATH` holds neither.
    BesideHerdr,
    /// In `<home>/.local/bin`, and `PATH` holds neither.
    HomeLocalBin,
}

fn sync_pass_with(
    name: &str,
    panes: impl Fn(&std::path::Path) -> Vec<serde_json::Value>,
    ank_at: AnkAt,
) -> Vec<support::Call> {
    let dir = tempdir(name);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let herdr = support::link(&bin, "herdr");
    let home = dir.join("home");
    let ank_dir = match ank_at {
        AnkAt::OnPath | AnkAt::BesideHerdr => bin.clone(),
        AnkAt::HomeLocalBin => home.join(".local").join("bin"),
    };
    fs::create_dir_all(&ank_dir).unwrap();
    support::link(&ank_dir, "ank");
    let path = if ank_at == AnkAt::OnPath {
        bin.clone()
    } else {
        let empty = dir.join("empty");
        fs::create_dir_all(&empty).unwrap();
        empty
    };
    let repo = dir.join("repo");
    fs::create_dir_all(repo.join(".ank")).unwrap();
    let panes = panes(&repo);
    let agents: Vec<_> = panes
        .iter()
        .filter(|p| p.get("agent").is_some())
        .map(|p| {
            let mut agent = p.clone();
            agent["name"] = "ank-2".into();
            agent
        })
        .collect();
    let panes = serde_json::json!({ "id": "cli:pane:list",
        "result": { "type": "pane_list", "panes": panes } });
    let agents = serde_json::json!({ "id": "cli:agent:list",
        "result": { "type": "agent_list", "agents": agents } });
    let find = serde_json::json!({
        "contract": 1, "corpus": null, "total": 1, "shown": 1, "hidden": 0,
        "results": [{
            "id": "TASK-416cdde27bfb", "kind": "task", "status": "in_progress",
            "state": "claimed:marie@box/ank-2", "title": "Sync engine",
            "created": "2026-09-25T16:54:25Z", "archived": false,
        }],
    });
    let context = serde_json::json!({
        "contract": 1, "mode": "orientation", "head": null, "criteria": null,
        "method": null, "constraints": [], "proposed": [], "specs": [], "tasks": [],
        "log": [], "ready": 3, "blocked": 0, "finished_elsewhere": 0, "warnings": [],
    });
    // queue 1, and expiries ank cannot parse, so no ank_expires
    let status = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ank/status.json"),
    )
    .unwrap();
    let answers = [
        support::answer(&["pane", "list"], &panes.to_string(), "", 0),
        support::answer(&["agent", "list"], &agents.to_string(), "", 0),
        support::answer(&["find"], &find.to_string(), "", 0),
        support::answer(&["context"], &context.to_string(), "", 0),
        support::answer(&["status"], &status, "", 0),
    ];
    support::write_answers(&bin, &answers);
    support::write_answers(&ank_dir, &answers);

    let output = Command::new(env!("CARGO_BIN_EXE_herdr-ank"))
        .arg("sync")
        .env("PATH", &path)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("HERDR_BIN_PATH", &herdr)
        .env("HERDR_SOCKET_PATH", dir.join("herdr.sock"))
        .env("HERDR_PLUGIN_CONFIG_DIR", &dir)
        .env("HERDR_PLUGIN_STATE_DIR", &dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut calls = support::calls(&bin);
    if ank_dir != bin {
        calls.extend(support::calls(&ank_dir));
    }
    calls
}

fn call<'a>(calls: &'a [support::Call], prefix: &[&str]) -> &'a support::Call {
    calls
        .iter()
        .find(|c| c.argv.iter().zip(prefix).all(|(a, p)| a == p) && c.argv.len() >= prefix.len())
        .unwrap_or_else(|| panic!("no {prefix:?} among {calls:#?}"))
}

#[test]
fn a_sync_reports_the_agent_label_with_the_tokens() {
    let calls = sync_pass("sync-label", |repo| {
        vec![serde_json::json!({
            "pane_id": "w1:p2", "workspace_id": "w1", "tab_id": "w1:t1",
            "cwd": repo, "agent": "claude", "agent_status": "working",
            "focused": false, "revision": 1,
        })]
    });

    assert_eq!(
        call(&calls, &["pane", "report-metadata"]).argv,
        [
            "pane",
            "report-metadata",
            "w1:p2",
            "--source",
            "ank:sync",
            "--display-agent",
            "claude · TASK-416c",
            "--token",
            "ank_task=TASK-416c",
            "--token",
            "ank_title=Sync engine",
            "--token",
            "ank_queue=3",
            "--clear-token",
            "ank_expires",
            "--clear-token",
            "ank_ambiguous",
            "--ttl-ms",
            "90000",
        ]
    );
}

#[test]
fn a_sync_reports_the_workspace_counts_even_without_an_agent_pane() {
    let calls = sync_pass("sync-workspace", |repo| {
        vec![serde_json::json!({
            "pane_id": "w4:p1", "workspace_id": "w4", "tab_id": "w4:t1",
            "cwd": repo.join("src"), "focused": false, "revision": 1,
        })]
    });

    assert_eq!(
        call(&calls, &["workspace", "report-metadata"]).argv,
        [
            "workspace",
            "report-metadata",
            "w4",
            "--source",
            "ank:sync",
            "--token",
            "ank_queue=3",
            "--token",
            "ank_claims=1",
            "--token",
            "ank_review=1",
            "--ttl-ms",
            "90000",
        ]
    );
    assert!(
        calls
            .iter()
            .all(|c| c.argv[0] != "pane" || c.argv[1] != "report-metadata"),
        "a pane without an agent got a pane report: {calls:#?}"
    );
}

/// The calls the fake recorded in `bin`, without a last line it is still
/// writing: the daemon runs while the test reads.
fn calls_so_far(bin: &std::path::Path) -> Vec<support::Call> {
    fs::read_to_string(bin.join("calls.jsonl"))
        .unwrap_or_default()
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// A daemon the test started, killed however the test ends: a panic must not
/// leave it running.
struct Running(std::process::Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `herdr-ank daemon` on `state_dir` against the fake herdr of `bin/`,
/// waits for its first sync, which comes after the welcome, and stops it.
/// Its stderr is the signal: the fakes the daemon runs at once may garble a
/// line of `calls.jsonl`, but the welcome goes out before any of them.
fn daemon_start(state_dir: &std::path::Path, bin: &std::path::Path) {
    let herdr = bin.join(format!("herdr{}", std::env::consts::EXE_SUFFIX));
    let err = state_dir.with_file_name("daemon.err");
    let mut daemon = Running(
        Command::new(env!("CARGO_BIN_EXE_herdr-ank"))
            .arg("daemon")
            .env("PATH", bin)
            .env("HERDR_BIN_PATH", &herdr)
            .env("HERDR_SOCKET_PATH", state_dir.join("absent.sock"))
            .env("HERDR_PLUGIN_CONFIG_DIR", state_dir)
            .env("HERDR_PLUGIN_STATE_DIR", state_dir)
            .stdout(Stdio::null())
            .stderr(fs::File::create(&err).unwrap())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    let synced = loop {
        let said = fs::read_to_string(&err).unwrap_or_default();
        if said.contains("herdr-ank daemon: sync") {
            break true;
        }
        if daemon.0.try_wait().unwrap().is_some() || started.elapsed() > Duration::from_secs(10) {
            break false;
        }
        std::thread::sleep(ms(20));
    };
    drop(daemon);
    assert!(
        synced,
        "the daemon never ran its first sync; it said: {}",
        fs::read_to_string(&err).unwrap_or_default()
    );
}

/// A state dir and a `bin/` holding the fake herdr and ank; `notification
/// show` answers `notify_code`.
fn welcome_setup(name: &str, notify_code: u8) -> (PathBuf, PathBuf) {
    let dir = tempdir(name);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    support::link(&bin, "herdr");
    support::link(&bin, "ank");
    let refused = r#"{"error":{"code":"internal","message":"no"},"id":"cli:notification:show"}"#;
    support::write_answers(
        &bin,
        &[support::answer(
            &["notification", "show"],
            "",
            if notify_code == 0 { "" } else { refused },
            notify_code,
        )],
    );
    let state = dir.join("state");
    fs::create_dir_all(&state).unwrap();
    (state, bin)
}

/// The welcomes herdr was asked for so far.
fn notifications(bin: &std::path::Path) -> Vec<support::Call> {
    calls_so_far(bin)
        .into_iter()
        .filter(|c| c.argv.starts_with(&["notification".into(), "show".into()]))
        .collect()
}

fn state_files(state: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(state)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != "daemon.lock")
        .collect();
    names.sort();
    names
}

#[test]
fn the_first_daemon_start_sends_one_welcome_naming_the_sidebar_tokens() {
    let (state, bin) = welcome_setup("welcome-first", 0);
    daemon_start(&state, &bin);

    let told = notifications(&bin);
    assert_eq!(told.len(), 1, "{told:#?}");
    let argv = &told[0].argv;
    let title = &argv[2];
    assert!(title.contains("ank"), "{argv:?}");
    let body = &argv[argv.iter().position(|a| a == "--body").expect("a body") + 1];
    for token in [
        "$ank_task",
        "$ank_expires",
        "$ank_title",
        "README",
        "Sidebar",
    ] {
        assert!(body.contains(token), "the body names {token}: {body}");
    }
    assert_eq!(state_files(&state), ["welcomed"], "a marker is left");
}

#[test]
fn a_later_daemon_start_sends_no_welcome() {
    let (state, bin) = welcome_setup("welcome-again", 0);
    daemon_start(&state, &bin);
    daemon_start(&state, &bin);
    assert_eq!(notifications(&bin).len(), 1, "one welcome over two starts");
}

#[test]
fn a_welcome_herdr_refused_leaves_no_marker_and_is_sent_again() {
    let (state, bin) = welcome_setup("welcome-refused", 1);
    daemon_start(&state, &bin);
    assert_eq!(notifications(&bin).len(), 1);
    assert!(state_files(&state).is_empty(), "{:?}", state_files(&state));

    daemon_start(&state, &bin);
    assert_eq!(notifications(&bin).len(), 2, "sent again at the next start");
}

fn one_agent_pane(repo: &std::path::Path) -> Vec<serde_json::Value> {
    vec![serde_json::json!({
        "pane_id": "w1:p2", "workspace_id": "w1", "tab_id": "w1:t1",
        "cwd": repo, "agent": "claude", "agent_status": "working",
        "focused": false, "revision": 1,
    })]
}

#[test]
fn a_sync_finds_ank_beside_herdr_when_path_lacks_it() {
    let calls = sync_pass_with("ank-beside-herdr", one_agent_pane, AnkAt::BesideHerdr);
    call(&calls, &["find"]);
    assert!(call(&calls, &["pane", "report-metadata"])
        .argv
        .contains(&"claude · TASK-416c".to_string()));
}

#[test]
fn a_sync_finds_ank_in_the_home_local_bin_when_path_lacks_it() {
    let calls = sync_pass_with("ank-home-local", one_agent_pane, AnkAt::HomeLocalBin);
    call(&calls, &["find"]);
    assert!(call(&calls, &["pane", "report-metadata"])
        .argv
        .contains(&"claude · TASK-416c".to_string()));
}
