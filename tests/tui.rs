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
