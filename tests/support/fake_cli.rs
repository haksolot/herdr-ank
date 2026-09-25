//! A stand-in for `herdr` or `ank` in the integration tests, the same on every
//! OS (ADR-599b6f424271): cargo builds it, a test links it under the name the
//! code runs, and it records each call and replays an answer. No shell.
//!
//! Its home is the directory it is linked in or, when that directory holds a
//! file `home-from-last-arg`, the parent of its last argument (`ank ...
//! --repo <home>/repo`). There it appends the call to `calls.jsonl`, then
//! answers with the first rule of `answers.json` whose `args` start its argv
//! and whose `agent`, when given, says whether `ANK_AGENT` is set. No rule
//! matching is an empty answer and exit 0.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    agent: Option<bool>,
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stderr: String,
    #[serde(default)]
    code: u8,
    /// A file to copy, `[from, to]`, before answering.
    #[serde(default)]
    copy: Option<(PathBuf, PathBuf)>,
}

fn home(argv: &[String]) -> PathBuf {
    let exe = std::env::current_exe().expect("the fake knows where it lives");
    let dir = exe.parent().expect("the fake lives in a directory");
    if dir.join("home-from-last-arg").is_file() {
        if let Some(parent) = argv.last().and_then(|last| Path::new(last).parent()) {
            return parent.to_path_buf();
        }
    }
    dir.to_path_buf()
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let agent = std::env::var("ANK_AGENT").ok().filter(|a| !a.is_empty());
    let home = home(&argv);

    let call = serde_json::json!({ "argv": argv, "ank_agent": agent });
    let mut calls = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("calls.jsonl"))
        .expect("the fake records its call");
    writeln!(calls, "{call}").unwrap();

    let answers: Vec<Answer> = fs::read_to_string(home.join("answers.json"))
        .map(|text| serde_json::from_str(&text).expect("answers.json is a list of answers"))
        .unwrap_or_default();
    let Some(answer) = answers
        .into_iter()
        .find(|a| argv.starts_with(&a.args) && a.agent.is_none_or(|want| want == agent.is_some()))
    else {
        return ExitCode::SUCCESS;
    };
    if let Some((from, to)) = &answer.copy {
        fs::copy(from, to).expect("the fake copies what its answer names");
    }
    print!("{}", answer.stdout);
    eprint!("{}", answer.stderr);
    ExitCode::from(answer.code)
}
