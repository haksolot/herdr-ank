//! The herdr boundary (ADR-357c017baf9b): every action is an argv handed to a
//! fake `herdr` (tests/support) that records it, and the event stream is a fixed NDJSON file,
//! replayed through an in-memory stream on every OS and served over a real
//! Unix socket on Unix (ADR-599b6f424271).

use std::cell::RefCell;
use std::fs;
#[cfg(unix)]
use std::io::{BufRead, BufReader};
use std::io::{Cursor, Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};

use herdr_ank::herdr::{
    subscribe_over, Client, Event, HerdrError, Sound, Subscription, TabCreate, WorktreeCreate,
};

mod support;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn tempdir(name: &str) -> PathBuf {
    let n = NEXT_DIR.fetch_add(1, Ordering::SeqCst);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
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

/// A fake `herdr` (tests/support/fake_cli.rs): records its argv, then answers
/// like the real binary does, `stdout` and exit 0 or `stderr` and exit 1.
struct FakeHerdr {
    dir: PathBuf,
    bin: PathBuf,
}

impl FakeHerdr {
    fn answering(name: &str, response: &str) -> Self {
        Self::build(name, support::answer(&[], response, "", 0))
    }

    fn failing(name: &str, response: &str) -> Self {
        Self::build(name, support::answer(&[], "", response, 1))
    }

    fn build(name: &str, answer: serde_json::Value) -> Self {
        let dir = tempdir(name);
        let bin = support::link(&dir, "herdr");
        support::write_answers(&dir, &[answer]);
        FakeHerdr { dir, bin }
    }

    fn client(&self) -> Client {
        Client::new(&self.bin, self.dir.join("herdr.sock"))
    }

    fn argv(&self) -> Vec<String> {
        support::calls(&self.dir)
            .pop()
            .expect("the fake herdr was never run")
            .argv
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
            "ank:sync",
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
            "ank:sync",
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
        .agent_start("ank-2", "claude", "w1:p2", &[])
        .unwrap();
    assert_eq!(
        fake.argv(),
        ["agent", "start", "ank-2", "--kind", "claude", "--pane", "w1:p2"]
    );
    assert_eq!(agent.pane_id, "w1:p2");
}

#[test]
fn agent_start_passes_agent_args_after_a_double_dash() {
    let fake = FakeHerdr::answering(
        "agent-start-args",
        &format!(
            r#"{{"id":"cli:agent:start","result":{{"type":"agent_started","argv":["claude"],"agent":{PANE}}}}}"#
        ),
    );
    fake.client()
        .agent_start(
            "ank-2",
            "claude",
            "w1:p2",
            &["--model".into(), "opus".into(), "--kind x".into()],
        )
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "agent", "start", "ank-2", "--kind", "claude", "--pane", "w1:p2", "--", "--model",
            "opus", "--kind x"
        ]
    );
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

/// herdr's end of the socket, in memory: what it will answer, and what it
/// was sent.
struct Replay {
    answer: Cursor<Vec<u8>>,
    sent: Rc<RefCell<Vec<u8>>>,
}

impl Read for Replay {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.answer.read(buf)
    }
}

impl Write for Replay {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sent.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Subscribes over a stream that answers `fixture_name`, and hands back the
/// events read and the request sent. Runs on every OS.
fn replay(
    fixture_name: &str,
    subscriptions: &[Subscription],
) -> (Result<Vec<Event>, HerdrError>, String) {
    let sent = Rc::new(RefCell::new(Vec::new()));
    let stream = Replay {
        answer: Cursor::new(fs::read(fixture(fixture_name)).unwrap()),
        sent: Rc::clone(&sent),
    };
    let events = subscribe_over(stream, "the replay", subscriptions).map(Iterator::collect);
    let request = String::from_utf8(sent.borrow().clone()).unwrap();
    (events, request)
}

/// Serves `fixture` to the first client and hands back the request line it
/// sent, or an empty line when no client came within five seconds.
#[cfg(unix)]
fn serve(name: &str, fixture_name: &str) -> (Client, thread::JoinHandle<String>) {
    let dir = tempdir(name);
    // A socket path must fit sun_path, which the target directory may not.
    let socket = std::env::temp_dir().join(format!("herdr-ank-{}-{name}.sock", std::process::id()));
    let _ = fs::remove_file(&socket);
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
    let subs = [
        Subscription::new("pane.updated"),
        Subscription::for_pane("pane.agent_status_changed", "w1:p2"),
    ];
    let (events, request) = replay("events.ndjson", &subs);
    let events = events.unwrap();

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
    let events = replay("events.ndjson", &[Subscription::new("pane.updated")])
        .0
        .unwrap();

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
    let (result, _) = replay(
        "subscribe_error.ndjson",
        &[Subscription::new("pane.agent_status_changed")],
    );
    match result {
        Err(HerdrError::Api { code, message }) => {
            assert_eq!(code, "invalid_request");
            assert!(message.contains("pane_id"), "{message}");
        }
        Err(other) => panic!("expected HerdrError::Api, got {other:?}"),
        Ok(_) => panic!("expected HerdrError::Api, got a stream"),
    }
}

#[cfg(unix)]
#[test]
fn subscribe_over_a_unix_socket_sends_the_request_and_reads_the_stream() {
    let (client, server) = serve("subscribe-socket", "events.ndjson");
    let events: Vec<Event> = client
        .subscribe(&[Subscription::new("pane.updated")])
        .unwrap()
        .collect();
    let request = server.join().unwrap();

    let request: serde_json::Value = serde_json::from_str(&request).unwrap();
    assert_eq!(request["method"], "events.subscribe");
    assert_eq!(events.len(), 4, "{events:#?}");
}

#[cfg(windows)]
#[test]
fn subscribe_to_an_absent_named_pipe_is_a_herdr_error_naming_the_pipe() {
    let pipe = format!(r"\\.\pipe\herdr-ank-test-absent-{}", std::process::id());
    let client = Client::new("herdr.exe", &pipe);
    match client.subscribe(&[Subscription::new("pane.updated")]) {
        Err(err @ HerdrError::Io { .. }) => {
            assert!(err.to_string().contains(&pipe), "{err}")
        }
        Err(other) => panic!("expected HerdrError::Io, got {other:?}"),
        Ok(_) => panic!("expected HerdrError::Io, got a stream"),
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

#[test]
fn plugin_pane_open_names_the_entrypoint_and_returns_the_opened_pane() {
    let fake = FakeHerdr::answering(
        "plugin-pane-open",
        &format!(
            r#"{{"id":"cli:plugin","result":{{"type":"plugin_pane_opened","plugin_pane":{{"plugin_id":"ank","entrypoint":"pick","pane":{PANE},"opened_by":"a field herdr added later"}}}}}}"#
        ),
    );
    let opened = fake
        .client()
        .plugin_pane_open("pick", Some("w1"), &[("ANK_WORK", "1"), ("A", "b=c")])
        .unwrap();
    assert_eq!(
        fake.argv(),
        [
            "plugin",
            "pane",
            "open",
            "--plugin",
            "ank",
            "--entrypoint",
            "pick",
            "--workspace",
            "w1",
            "--env",
            "ANK_WORK=1",
            "--env",
            "A=b=c",
            "--focus"
        ]
    );
    assert_eq!(opened.entrypoint, "pick");
    assert_eq!(opened.pane.pane_id, "w1:p2");
}

#[test]
fn plugin_pane_open_without_workspace_or_env_passes_neither() {
    let fake = FakeHerdr::answering(
        "plugin-pane-open-bare",
        &format!(
            r#"{{"id":"cli:plugin","result":{{"type":"plugin_pane_opened","plugin_pane":{{"plugin_id":"ank","entrypoint":"tui","pane":{PANE}}}}}}}"#
        ),
    );
    fake.client().plugin_pane_open("tui", None, &[]).unwrap();
    assert_eq!(
        fake.argv(),
        [
            "plugin",
            "pane",
            "open",
            "--plugin",
            "ank",
            "--entrypoint",
            "tui",
            "--focus"
        ]
    );
}

#[test]
fn plugin_pane_open_refused_is_an_api_error() {
    // As herdr 0.9.1 answers an unknown entrypoint: the body on stderr, exit 1.
    let fake = FakeHerdr::failing(
        "plugin-pane-open-error",
        r#"{"error":{"code":"plugin_pane_not_found","message":"plugin pane entrypoint 'nope' not found"},"id":"cli:plugin"}"#,
    );
    match fake.client().plugin_pane_open("nope", None, &[]) {
        Err(HerdrError::Api { code, .. }) => assert_eq!(code, "plugin_pane_not_found"),
        other => panic!("expected an Api error, got {other:?}"),
    }
}

#[test]
fn subscribe_decodes_the_lifecycle_events_the_daemon_wakes_on() {
    let events = replay(
        "lifecycle_events.ndjson",
        &[Subscription::new("worktree.created")],
    )
    .0
    .unwrap();

    assert_eq!(
        events,
        [
            Event::PaneAgentDetected {
                pane_id: "wN:p5".into(),
                workspace_id: "wN".into(),
                agent: Some("claude".into()),
            },
            Event::WorktreeCreated {
                workspace_id: "wN".into(),
                path: PathBuf::from("/wt/task-91a1"),
                branch: Some("task/91a1".into()),
            },
            Event::WorktreeOpened {
                workspace_id: "wN".into(),
                path: PathBuf::from("/wt/task-91a1"),
                branch: Some("task/91a1".into()),
            },
            Event::WorktreeRemoved {
                workspace_id: "wN".into(),
                path: PathBuf::from("/wt/task-91a1"),
            },
            Event::WorkspaceClosed {
                workspace_id: "wN".into(),
            },
        ]
    );
}

#[test]
fn a_verb_whose_result_is_not_read_succeeds_on_an_empty_stdout() {
    // herdr 0.9.1 `pane report-metadata` exits 0 and writes nothing.
    let fake = FakeHerdr::answering("empty-report", "");
    fake.client()
        .report_metadata("w1:p2", &[("ank_task", "TASK-b636")], &[], Some(90_000))
        .expect("an empty stdout with exit 0 is a success");
    let fake = FakeHerdr::answering("empty-notify", "");
    fake.client()
        .notification_show("t", None, Sound::None)
        .expect("an empty stdout with exit 0 is a success");
    let fake = FakeHerdr::answering("empty-prompt", "");
    fake.client()
        .agent_prompt("ank-2", "ank claim TASK-b636")
        .expect("an empty stdout with exit 0 is a success");
}

#[test]
fn a_verb_whose_result_is_read_still_fails_on_an_empty_stdout() {
    let fake = FakeHerdr::answering("empty-list", "");
    assert!(matches!(
        fake.client().pane_list(None),
        Err(HerdrError::Decode(_))
    ));
}
