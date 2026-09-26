//! `herdr-ank tui` hands the pane to `ank tui` and exits as it does, on every
//! OS: by exec on Unix, as a waited child on Windows (ADR-599b6f424271).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod support;

fn scratch(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tui-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_tui_runs_ank_tui_in_the_corpus_and_exits_with_its_code() {
    let dir = scratch("exit-code");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    support::link(&bin, "ank");
    support::write_answers(&bin, &[support::answer(&["tui"], "", "", 3)]);
    let corpus = dir.join("repo");
    fs::create_dir_all(corpus.join(".ank")).unwrap();
    fs::create_dir_all(corpus.join("src")).unwrap();
    let context = serde_json::json!({ "workspace_cwd": corpus.join("src") }).to_string();

    let status = Command::new(env!("CARGO_BIN_EXE_herdr-ank"))
        .arg("tui")
        .env("PATH", &bin)
        .env("HERDR_PLUGIN_CONTEXT_JSON", context)
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(3));
    let calls = support::calls(&bin);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].argv, ["tui"]);
}

/// What herdr 0.9.1 prints for `plugin pane open` of the tui overlay.
const TUI_OPENED: &str = r#"{"id":"cli:plugin","result":{"type":"plugin_pane_opened","plugin_pane":{"plugin_id":"ank","entrypoint":"tui","pane":{"pane_id":"w7:p3","workspace_id":"w7","tab_id":"w7:t1","agent_status":"unknown","focused":true,"revision":1}}}}"#;

fn open(dir: &Path, context: Option<&str>) -> std::process::Output {
    let bin = dir.join("bin");
    let mut command = Command::new(env!("CARGO_BIN_EXE_herdr-ank"));
    command
        .arg("open")
        .env(
            "HERDR_BIN_PATH",
            bin.join(format!("herdr{}", std::env::consts::EXE_SUFFIX)),
        )
        .env("HERDR_SOCKET_PATH", dir.join("herdr.sock"))
        .env_remove("HERDR_PLUGIN_CONTEXT_JSON");
    if let Some(context) = context {
        command.env("HERDR_PLUGIN_CONTEXT_JSON", context);
    }
    command.output().unwrap()
}

fn fake_herdr(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    support::link(&bin, "herdr");
    support::write_answers(
        &bin,
        &[support::answer(
            &["plugin", "pane", "open"],
            TUI_OPENED,
            "",
            0,
        )],
    );
    bin
}

#[test]
fn open_asks_herdr_for_the_tui_overlay_of_the_invoking_workspace() {
    let dir = scratch("open");
    let bin = fake_herdr(&dir);
    let context = r#"{"workspace_id":"w7","workspace_cwd":"/nowhere","later_field":1}"#;

    let output = open(&dir, Some(context));

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = support::calls(&bin);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(
        calls[0].argv,
        [
            "plugin",
            "pane",
            "open",
            "--plugin",
            "ank",
            "--entrypoint",
            "tui",
            "--workspace",
            "w7",
            "--focus"
        ]
    );
}

#[test]
fn open_without_a_workspace_exits_1_naming_it_and_calls_nothing() {
    for context in [None, Some(r#"{"workspace_cwd":"/home/me/repo"}"#)] {
        let dir = scratch(&format!("open-none-{}", context.is_some()));
        let bin = fake_herdr(&dir);

        let output = open(&dir, context);

        assert_eq!(output.status.code(), Some(1), "context {context:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("workspace"), "{stderr}");
        assert!(support::calls(&bin).is_empty());
    }
}

#[test]
fn the_manifest_declares_the_open_action_on_the_workspace() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("herdr-plugin.toml");
    let manifest: toml::Table = fs::read_to_string(path).unwrap().parse().unwrap();
    let actions = manifest["actions"].as_array().unwrap();
    let open = actions
        .iter()
        .find(|a| a["id"].as_str() == Some("open"))
        .expect("herdr-plugin.toml declares no [[actions]] with id \"open\"");

    assert_eq!(open["title"].as_str(), Some("Ouvrir ank"));
    assert_eq!(
        open["contexts"],
        toml::Value::Array(vec!["workspace".into()])
    );
    assert_eq!(
        open["command"],
        toml::Value::Array(vec!["./bin/herdr-ank".into(), "open".into()])
    );
}
