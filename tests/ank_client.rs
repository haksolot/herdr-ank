use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use herdr_ank::ank::{AnkError, Client, ExitKind};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ank");

/// A throwaway directory per test, under the target directory.
fn scratch(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "ank-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The fake `ank`: records its argv, prints `stdout` and `stderr`, and exits with the
/// code in `code`, all read from the parent of the `--repo` it is handed. It is
/// written once, before any test spawns it: writing an executable while another
/// thread forks makes its exec fail with ETXTBSY.
fn fake_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let bin = scratch("bin").join("ank");
        let script = "#!/bin/sh\nfor last; do :; done\nd=$(dirname \"$last\")\n\
                      printf '%s\\n' \"$@\" > \"$d/argv\"\ncat \"$d/stdout\"\n\
                      cat \"$d/stderr\" >&2\nexit $(cat \"$d/code\")\n";
        let mut file = fs::File::create(&bin).unwrap();
        file.write_all(script.as_bytes()).unwrap();
        file.sync_all().unwrap();
        drop(file);
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        bin
    })
}

/// A client over `dir/repo` whose fake `ank` answers with `stdout`, `stderr` and `code`.
fn fake_ank(dir: &Path, stdout: &str, stderr: &str, code: i32) -> Client {
    fs::write(dir.join("stdout"), stdout).unwrap();
    fs::write(dir.join("stderr"), stderr).unwrap();
    fs::write(dir.join("code"), code.to_string()).unwrap();
    let repo = dir.join("repo");
    fs::create_dir_all(&repo).unwrap();
    Client::with_program(fake_bin(), &repo)
}

fn argv(dir: &Path) -> Vec<String> {
    fs::read_to_string(dir.join("argv"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn fixture(verb: &str) -> String {
    fs::read_to_string(format!("{FIXTURES}/{verb}.json")).unwrap()
}

/// The same fixture with a field no contract declares, at the top and in a nested object.
fn with_unknown_fields(json: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    add_unknown(&mut value);
    value.to_string()
}

fn add_unknown(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for v in map.values_mut() {
                add_unknown(v);
            }
            map.insert("from_the_future".into(), serde_json::json!({"x": [1]}));
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(add_unknown),
        _ => {}
    }
}

fn client_over(verb: &str, stdout: &str) -> (Client, PathBuf, PathBuf) {
    let dir = scratch(verb);
    let client = fake_ank(&dir, stdout, "", 0);
    let repo = dir.join("repo");
    (client, dir, repo)
}

#[test]
fn status_runs_the_verb_and_reads_the_fixture() {
    let (client, dir, repo) = client_over("status", &fixture("status"));
    let status = client.status().unwrap();
    assert_eq!(
        argv(&dir),
        ["status", "--json", "--repo", repo.to_str().unwrap()]
    );
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!(status.identity.value, "claude-code/1.0.0");
    assert_eq!(status.claim.unwrap().id, "TASK-000000000002");
    assert_eq!(status.elsewhere[0].holder.as_deref(), Some("human:marie"));
    assert_eq!(status.signals, 5);
}

#[test]
fn find_passes_its_arguments_and_reads_the_fixture() {
    let (client, dir, repo) = client_over("find", &fixture("find"));
    let found = client.find(&["--status", "in_progress"]).unwrap();
    assert_eq!(
        argv(&dir),
        [
            "find",
            "--status",
            "in_progress",
            "--json",
            "--repo",
            repo.to_str().unwrap()
        ]
    );
    assert_eq!(found.total, 6);
    assert_eq!(found.results.len(), 6);
    assert_eq!(found.results[4].id, "ADR-0000000000ab");
    assert_eq!(found.results[4].kind, "adr");
    assert_eq!(found.results[0].state, "open");
}

#[test]
fn show_names_the_entity_and_reads_the_fixture() {
    let (client, dir, repo) = client_over("show", &fixture("show"));
    let shown = client.show("TASK-000000000001").unwrap();
    assert_eq!(
        argv(&dir),
        [
            "show",
            "TASK-000000000001",
            "--json",
            "--repo",
            repo.to_str().unwrap()
        ]
    );
    assert_eq!(shown.id, "TASK-000000000001");
    assert_eq!(shown.coordination, None);
    assert_eq!(shown.blocked_by[0].id, "TASK-000000000002");
    assert_eq!(shown.unblocks[0].status.as_deref(), Some("open"));
    assert!(shown.content.starts_with("---\nid: TASK-000000000001\n"));
}

#[test]
fn context_takes_an_optional_path_and_reads_the_fixture() {
    let (client, dir, repo) = client_over("context", &fixture("context"));
    let context = client.context(Some(Path::new("src"))).unwrap();
    assert_eq!(
        argv(&dir),
        ["context", "src", "--json", "--repo", repo.to_str().unwrap()]
    );
    assert_eq!(context.constraints[0].id, "ADR-0000000000ab");
    assert_eq!(context.proposed[0].id, "ADR-0000000000ba");
    assert_eq!(context.tasks.len(), 4);
    assert!(context.tasks[0].ready);
    assert_eq!(context.ready, 2);

    client.context(None).unwrap();
    assert_eq!(
        argv(&dir),
        ["context", "--json", "--repo", repo.to_str().unwrap()]
    );
}

#[test]
fn an_unknown_field_in_any_fixture_is_ignored() {
    let (client, _, _) = client_over("status", &with_unknown_fields(&fixture("status")));
    assert_eq!(client.status().unwrap().signals, 5);
    let (client, _, _) = client_over("find", &with_unknown_fields(&fixture("find")));
    assert_eq!(client.find(&[]).unwrap().total, 6);
    let (client, _, _) = client_over("show", &with_unknown_fields(&fixture("show")));
    assert_eq!(client.show("TASK-0001").unwrap().log_total, 1);
    let (client, _, _) = client_over("context", &with_unknown_fields(&fixture("context")));
    assert_eq!(client.context(None).unwrap().blocked, 2);
}

#[test]
fn a_contract_other_than_1_is_refused_by_name() {
    let bumped = fixture("status").replacen("\"contract\":1", "\"contract\":2", 1);
    let (client, _, _) = client_over("status", &bumped);
    match client.status() {
        Err(AnkError::Contract { found }) => assert_eq!(found, Some(2)),
        other => panic!("expected a contract refusal, got {other:?}"),
    }

    let (client, _, _) = client_over("status", r#"{"branch":"main"}"#);
    assert!(matches!(
        client.status(),
        Err(AnkError::Contract { found: None })
    ));
}

#[test]
fn every_exit_code_is_routed_with_its_stderr() {
    let expected = [
        (1, ExitKind::Generic),
        (2, ExitKind::NotFound),
        (3, ExitKind::Conflict),
        (4, ExitKind::Unavailable),
        (5, ExitKind::Proof),
        (6, ExitKind::Transition),
        (7, ExitKind::Prerequisite),
        (8, ExitKind::Findings),
        (9, ExitKind::Environment),
    ];
    for (code, kind) in expected {
        let dir = scratch("exit");
        let stderr = format!("error[{code}]: refused\n  -> ank context\n");
        match fake_ank(&dir, "", &stderr, code).show("TASK-9999") {
            Err(AnkError::Exit {
                code: got,
                kind: got_kind,
                stderr: got_stderr,
            }) => {
                assert_eq!(got, code as u8);
                assert_eq!(got_kind, kind);
                assert_eq!(got_stderr, stderr);
            }
            other => panic!("exit {code}: expected an Exit error, got {other:?}"),
        }
    }
    assert_ne!(ExitKind::Conflict, ExitKind::Unavailable);
    assert_ne!(ExitKind::Transition, ExitKind::Prerequisite);
}

#[test]
fn stdout_is_not_parsed_when_the_code_is_not_zero() {
    let dir = scratch("refusal");
    let err = fake_ank(&dir, &fixture("status"), "error[9]: git absent\n", 9)
        .status()
        .unwrap_err();
    assert!(err.is_environment());
    assert!(matches!(err, AnkError::Exit { code: 9, .. }));
}

#[test]
fn a_missing_binary_is_an_error_not_a_panic() {
    let dir = scratch("missing");
    fake_bin();
    let err = Client::with_program(dir.join("no-such-ank"), &dir)
        .status()
        .unwrap_err();
    assert!(matches!(err, AnkError::Spawn(_)));
}

#[test]
fn new_runs_the_ank_on_the_path() {
    assert_eq!(
        Client::new(Path::new("/some/repo")).program(),
        Path::new("ank")
    );
}
