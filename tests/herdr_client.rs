//! The herdr boundary (ADR-357c017baf9b): every action is an argv handed to a
//! fake `herdr` that records it, and the event stream is a fixed NDJSON file
//! served over a real Unix socket.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use herdr_ank::herdr::{Client, Event, HerdrError, Sound, Subscription, TabCreate, WorktreeCreate};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn tempdir(name: &str) -> PathBuf {
    let n = NEXT_DIR.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "herdr-ank-test-{}-{}-{}",
        std::process::id(),
        name,
        n
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/herdr")
        .join(name)
}

/// A fake `herdr`: writes its argv, NUL-separated, then answers like the real
/// binary does, `stdout` and exit 0 or `stderr` and exit 1.
struct FakeHerdr {
    dir: PathBuf,
    bin: PathBuf,
}

impl FakeHerdr {
    fn answering(name: &str, response: &str) -> Self {
        Self::build(name, response, 0)
    }

    fn failing(name: &str, response: &str) -> Self {
        Self::build(name, response, 1)
    }

    fn build(name: &str, response: &str, exit: i32) -> Self {
        let dir = tempdir(name);
        let bin = dir.join("herdr");
        let response_file = dir.join("response.json");
        fs::write(&response_file, response).unwrap();
        let stream = if exit == 0 { "" } else { " >&2" };
        let script = format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\0' \"$a\"; done > '{argv}'\ncat '{resp}'{stream}\nexit {exit}\n",
            argv = dir.join("argv").display(),
            resp = response_file.display(),
        );
        fs::write(&bin, script).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        // A child forked by a parallel test between our open and its exec holds
        // the script's write descriptor for a moment, and exec fails with
        // ETXTBSY until it lets go. Wait that out here, not in the client.
        for _ in 0..500 {
            match std::process::Command::new(&bin).output() {
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    thread::sleep(Duration::from_millis(2))
                }
                _ => break,
            }
        }
        let _ = fs::remove_file(dir.join("argv"));
        FakeHerdr { dir, bin }
    }

    fn client(&self) -> Client {
        Client::new(&self.bin, self.dir.join("herdr.sock"))
    }

    fn argv(&self) -> Vec<String> {
        let raw = fs::read(self.dir.join("argv")).expect("the fake herdr was never run");
        raw.split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8(s.to_vec()).unwrap())
            .collect()
    }
}

const PANE: &str = r#"{"pane_id":"w1:p2","workspace_id":"w1","tab_id":"w1:t1","agent":"claude","agent_status":"working","cwd":"/src/repo","focused":false,"revision":3,"tokens":{"ank":"TASK-de0a"},"a_field_herdr_added_later":true}"#;

#[test]
fn from_env_reads_bin_and_socket_and_names_what_is_missing() {
    // The only test touching these variables, so no other test races it.
    std::env::remove_var("HERDR_BIN_PATH");
    std::env::set_var("HERDR_SOCKET_PATH", "/run/herdr.sock");
    match Client::from_env() {
        Err(HerdrError::MissingEnv(var)) => assert_eq!(var, "HERDR_BIN_PATH"),
        Err(other) => panic!("expected MissingEnv(HERDR_BIN_PATH), got {other}"),
        Ok(_) => panic!("expected MissingEnv(HERDR_BIN_PATH), got a client"),
    }

    let fake = FakeHerdr::answering(
        "from-env",
        r#"{"id":"cli:agent:list","result":{"type":"agent_list","agents":[]}}"#,
    );
    std::env::set_var("HERDR_BIN_PATH", &fake.bin);
    let client = Client::from_env().expect("both variables are set");
    assert_eq!(client.socket_path(), Path::new("/run/herdr.sock"));
    client.agent_list().unwrap();
    assert_eq!(fake.argv(), ["agent", "list"]);
}

#[test]
fn pane_list_passes_the_workspace_and_decodes_panes() {
    let fake = FakeHerdr::answering(
        "pane-list",
        &format!(r#"{{"id":"cli:pane:list","result":{{"type":"pane_list","panes":[{PANE}]}}}}"#),
    );
    let panes = fake.client().pane_list(Some("w1")).unwrap();
    assert_eq!(fake.argv(), ["pane", "list", "--workspace", "w1"]);
    assert_eq!(panes.len(), 1);
    assert_eq!(panes[0].pane_id, "w1:p2");
    assert_eq!(panes[0].workspace_id, "w1");
    assert_eq!(panes[0].agent.as_deref(), Some("claude"));
    assert_eq!(panes[0].agent_status.as_deref(), Some("working"));
    assert_eq!(
        panes[0].tokens.get("ank").map(String::as_str),
        Some("TASK-de0a")
    );
}

#[test]
fn pane_list_without_workspace_passes_no_flag() {
    let fake = FakeHerdr::answering(
        "pane-list-all",
        r#"{"id":"cli:pane:list","result":{"type":"pane_list","panes":[]}}"#,
    );
    assert!(fake.client().pane_list(None).unwrap().is_empty());
    assert_eq!(fake.argv(), ["pane", "list"]);
}

#[test]
fn agent_list_decodes_agents() {
    let fake = FakeHerdr::answering(
        "agent-list",
        &format!(r#"{{"id":"cli:agent:list","result":{{"type":"agent_list","agents":[{PANE}]}}}}"#),
    );
    let agents = fake.client().agent_list().unwrap();
    assert_eq!(fake.argv(), ["agent", "list"]);
    assert_eq!(agents[0].pane_id, "w1:p2");
}

#[test]
fn report_metadata_sets_and_clears_tokens_under_the_plugin_source() {
    let fake = FakeHerdr::answering(
        "report-metadata",
        r#"{"id":"cli:request","result":{"type":"ok"}}"#,
    );
    fake.client()
        .report_metadata(
            "w1:p2",
            &[("ank", "TASK-de0a"), ("ank_state", "claimed; stale")],
            &["ank_expiry"],
            Some(90_000),
        )
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "pane",
            "report-metadata",
            "w1:p2",
            "--source",
            "ank",
            "--token",
            "ank=TASK-de0a",
            "--token",
            "ank_state=claimed; stale",
            "--clear-token",
            "ank_expiry",
            "--ttl-ms",
            "90000",
        ]
    );
}

#[test]
fn report_metadata_without_ttl_passes_no_ttl() {
    let fake = FakeHerdr::answering(
        "report-metadata-nottl",
        r#"{"id":"cli:request","result":{"type":"ok"}}"#,
    );
    fake.client()
        .report_metadata("w1:p2", &[], &["ank"], None)
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "pane",
            "report-metadata",
            "w1:p2",
            "--source",
            "ank",
            "--clear-token",
            "ank"
        ]
    );
}

#[test]
fn notification_show_passes_title_body_and_sound() {
    let fake = FakeHerdr::answering(
        "notification",
        r#"{"id":"cli:notification:show","result":{"type":"notification_show","shown":true,"reason":"shown"}}"#,
    );
    fake.client()
        .notification_show(
            "TASK-de0a finished",
            Some("proof on task/de0a"),
            Sound::Done,
        )
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "notification",
            "show",
            "TASK-de0a finished",
            "--body",
            "proof on task/de0a",
            "--sound",
            "done",
        ]
    );
}

#[test]
fn worktree_create_passes_every_flag_and_returns_the_root_pane() {
    let fake = FakeHerdr::answering(
        "worktree",
        &format!(
            r#"{{"id":"cli:worktree:create","result":{{"type":"worktree_created","workspace":{{}},"tab":{{}},"worktree":{{}},"root_pane":{PANE}}}}}"#
        ),
    );
    let root = fake
        .client()
        .worktree_create(&WorktreeCreate {
            workspace: "w1",
            branch: "task/de0a",
            base: "main",
            path: Path::new("/wt/de0a"),
            label: Some("TASK-de0a"),
            focus: false,
        })
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "worktree",
            "create",
            "--workspace",
            "w1",
            "--branch",
            "task/de0a",
            "--base",
            "main",
            "--path",
            "/wt/de0a",
            "--label",
            "TASK-de0a",
            "--no-focus",
        ]
    );
    assert_eq!(root.pane_id, "w1:p2");
}

#[test]
fn tab_create_passes_cwd_env_label_and_focus() {
    let fake = FakeHerdr::answering(
        "tab",
        &format!(
            r#"{{"id":"cli:tab:create","result":{{"type":"tab_created","tab":{{}},"root_pane":{PANE}}}}}"#
        ),
    );
    let root = fake
        .client()
        .tab_create(&TabCreate {
            workspace: "w1",
            cwd: Path::new("/wt/de0a"),
            env: &[("ANK_AGENT", "me@host/ank-2")],
            label: Some("ank-2"),
            focus: true,
        })
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "tab",
            "create",
            "--workspace",
            "w1",
            "--cwd",
            "/wt/de0a",
            "--env",
            "ANK_AGENT=me@host/ank-2",
            "--label",
            "ank-2",
            "--focus",
        ]
    );
    assert_eq!(root.tab_id, "w1:t1");
}

#[test]
fn agent_start_names_kind_and_pane() {
    let fake = FakeHerdr::answering(
        "agent-start",
        &format!(
            r#"{{"id":"cli:agent:start","result":{{"type":"agent_started","argv":["claude"],"agent":{PANE}}}}}"#
        ),
    );
    let agent = fake
        .client()
        .agent_start("ank-2", "claude", "w1:p2")
        .unwrap();
    assert_eq!(
        fake.argv(),
        ["agent", "start", "ank-2", "--kind", "claude", "--pane", "w1:p2"]
    );
    assert_eq!(agent.pane_id, "w1:p2");
}

#[test]
fn agent_prompt_passes_the_text_as_one_argument_untouched_by_a_shell() {
    let fake = FakeHerdr::answering(
        "agent-prompt",
        &format!(
            r#"{{"id":"cli:agent:prompt","result":{{"type":"agent_prompted","agent":{PANE}}}}}"#
        ),
    );
    let text = "work TASK-de0a; then `ank done` && echo $HOME \"quoted\"";
    fake.client().agent_prompt("ank-2", text).unwrap();
    assert_eq!(fake.argv(), ["agent", "prompt", "ank-2", text]);
}

#[test]
fn a_cli_error_is_a_herdr_error_carrying_code_and_message() {
    let fake = FakeHerdr::failing(
        "cli-error",
        r#"{"error":{"code":"pane_not_found","message":"pane zz:p9 not found"},"id":"cli:request"}"#,
    );
    match fake
        .client()
        .report_metadata("zz:p9", &[("ank", "x")], &[], None)
    {
        Err(HerdrError::Api { code, message }) => {
            assert_eq!(code, "pane_not_found");
            assert_eq!(message, "pane zz:p9 not found");
        }
        other => panic!("expected HerdrError::Api, got {other:?}"),
    }
}

#[test]
fn a_cli_failure_without_json_is_a_herdr_error_not_a_panic() {
    let fake = FakeHerdr::failing("cli-garbage", "usage: herdr agent list\n");
    match fake.client().agent_list() {
        Err(HerdrError::Exit { status, stderr }) => {
            assert_eq!(status, Some(1));
            assert!(stderr.contains("usage"), "{stderr}");
        }
        other => panic!("expected HerdrError::Exit, got {other:?}"),
    }
}

#[test]
fn a_missing_binary_is_a_herdr_error() {
    let dir = tempdir("no-bin");
    let client = Client::new(dir.join("absent"), dir.join("herdr.sock"));
    assert!(matches!(client.agent_list(), Err(HerdrError::Io { .. })));
}

/// Serves `fixture` to the first client and hands back the request line it
/// sent, or an empty line when no client came within five seconds.
fn serve(name: &str, fixture_name: &str) -> (Client, thread::JoinHandle<String>) {
    let dir = tempdir(name);
    let socket = dir.join("herdr.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let body = fs::read(fixture(fixture_name)).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                Err(_) => return String::new(),
            }
        };
        stream.set_nonblocking(false).unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        let mut stream = stream;
        stream.write_all(&body).unwrap();
        request
    });
    (Client::new(dir.join("herdr"), socket), server)
}

#[test]
fn subscribe_sends_one_ndjson_events_subscribe_request() {
    let (client, server) = serve("subscribe-request", "events.ndjson");
    let subs = [
        Subscription::new("pane.updated"),
        Subscription::for_pane("pane.agent_status_changed", "w1:p2"),
    ];
    let events: Vec<Event> = client.subscribe(&subs).unwrap().collect();
    let request = server.join().unwrap();

    assert!(request.ends_with('\n'), "a request is one NDJSON line");
    let request: serde_json::Value = serde_json::from_str(&request).unwrap();
    assert_eq!(request["method"], "events.subscribe");
    assert!(request["id"].is_string());
    assert_eq!(
        request["params"],
        serde_json::json!({"subscriptions": [
            {"type": "pane.updated"},
            {"type": "pane.agent_status_changed", "pane_id": "w1:p2"},
        ]})
    );
    assert!(!events.is_empty());
}

#[test]
fn subscribe_decodes_the_fixed_stream_and_skips_the_unknown_type() {
    let (client, server) = serve("subscribe-decode", "events.ndjson");
    let events: Vec<Event> = client
        .subscribe(&[Subscription::new("pane.updated")])
        .unwrap()
        .collect();
    server.join().unwrap();

    assert_eq!(events.len(), 4, "{events:#?}");
    match &events[0] {
        Event::PaneUpdated(pane) => {
            assert_eq!(pane.pane_id, "wN:p2");
            assert_eq!(pane.tokens.get("probe").map(String::as_str), Some("1"));
        }
        other => panic!("expected PaneUpdated, got {other:?}"),
    }
    match &events[1] {
        Event::PaneAgentStatusChanged {
            pane_id,
            workspace_id,
            agent,
            agent_status,
        } => {
            assert_eq!(pane_id, "wN:p2");
            assert_eq!(workspace_id, "wN");
            assert_eq!(agent.as_deref(), Some("claude"));
            assert_eq!(agent_status, "done");
        }
        other => panic!("expected PaneAgentStatusChanged, got {other:?}"),
    }
    assert_eq!(
        events[2],
        Event::PaneClosed {
            pane_id: "wN:p3".into(),
            workspace_id: "wN".into()
        }
    );
    match &events[3] {
        Event::PaneCreated(pane) => assert_eq!(pane.pane_id, "wN:p4"),
        other => panic!("expected PaneCreated, got {other:?}"),
    }
}

#[test]
fn an_error_response_to_subscribe_is_a_herdr_error_not_a_panic() {
    let (client, server) = serve("subscribe-error", "subscribe_error.ndjson");
    let result = client.subscribe(&[Subscription::new("pane.agent_status_changed")]);
    server.join().unwrap();
    match result {
        Err(HerdrError::Api { code, message }) => {
            assert_eq!(code, "invalid_request");
            assert!(message.contains("pane_id"), "{message}");
        }
        Err(other) => panic!("expected HerdrError::Api, got {other:?}"),
        Ok(_) => panic!("expected HerdrError::Api, got a stream"),
    }
}

#[test]
fn subscribe_to_an_absent_socket_is_a_herdr_error() {
    let dir = tempdir("no-socket");
    let client = Client::new(dir.join("herdr"), dir.join("absent.sock"));
    assert!(matches!(
        client.subscribe(&[Subscription::new("pane.updated")]),
        Err(HerdrError::Io { .. })
    ));
}
