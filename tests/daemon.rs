//! The daemon's single instance and its coalescing (TASK-91a1, ADR-6fb76f3a1197).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use herdr_ank::daemon::{minutes_until, Lock, Schedule};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn tempdir(name: &str) -> PathBuf {
    let n = NEXT_DIR.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
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
