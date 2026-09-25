//! What the integration tests share to drive `fake-cli` (tests/support/fake_cli.rs).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

/// Links the fake as `dir/<name>` (`<name>.exe` on Windows) and returns that
/// path. A hard link opens no write handle, so a parallel test forking at the
/// same moment cannot make its exec fail with ETXTBSY; a copy is the fallback
/// when the link is refused.
pub fn link(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let fake = Path::new(env!("CARGO_BIN_EXE_fake-cli"));
    let _ = fs::remove_file(&path);
    if fs::hard_link(fake, &path).is_err() {
        fs::copy(fake, &path).unwrap();
    }
    path
}

/// Makes the fake linked in `dir` take the parent of its last argument as home.
pub fn home_from_last_arg(dir: &Path) {
    fs::write(dir.join("home-from-last-arg"), "").unwrap();
}

/// One rule of `answers.json`.
pub fn answer(args: &[&str], stdout: &str, stderr: &str, code: u8) -> Value {
    json!({ "args": args, "stdout": stdout, "stderr": stderr, "code": code })
}

pub fn write_answers(home: &Path, answers: &[Value]) {
    fs::write(
        home.join("answers.json"),
        Value::from(answers.to_vec()).to_string(),
    )
    .unwrap();
}

#[derive(Debug, Deserialize)]
pub struct Call {
    pub argv: Vec<String>,
    pub ank_agent: Option<String>,
}

/// Every call the fake recorded in `home`, oldest first.
pub fn calls(home: &Path) -> Vec<Call> {
    fs::read_to_string(home.join("calls.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
