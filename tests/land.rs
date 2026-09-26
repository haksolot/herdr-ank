//! Landing a task branch against real git: an integration tree on `main` and
//! the task's worktree on `task/abcd`, both in a temporary directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use herdr_ank::ank::Found;
use herdr_ank::land::{self, Landing, Request};

const BRANCH: &str = "task/abcd";

struct Repos {
    _dir: tempfile::TempDir,
    integration: PathBuf,
    worktree: PathBuf,
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

fn commit(dir: &Path, file: &str, content: &str, message: &str) {
    fs::write(dir.join(file), content).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", message]);
}

/// `main` holds `base` then `main.txt`; `task/abcd`, cut from `base` in its
/// own worktree, holds `task.txt`. The first rebase has something to rewrite.
fn repos() -> Repos {
    let dir = tempfile::tempdir().unwrap();
    let integration = dir.path().join("integration");
    let worktree = dir.path().join("wt");
    fs::create_dir_all(&integration).unwrap();
    git(&integration, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.name", "land test"),
        ("user.email", "land@test"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
    ] {
        git(&integration, &["config", key, value]);
    }
    commit(&integration, "base.txt", "base\n", "base");
    git(
        &integration,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            BRANCH,
            worktree.to_str().unwrap(),
        ],
    );
    commit(&integration, "main.txt", "main\n", "main moves");
    commit(&worktree, "task.txt", "task\n", "task work");
    Repos {
        _dir: dir,
        integration,
        worktree,
    }
}

fn found(status: &str) -> Found {
    Found {
        id: "TASK-abcd00000000".into(),
        kind: "task".into(),
        status: status.into(),
        state: status.into(),
        title: "a task".into(),
        created: "2026-09-26T00:00:00Z".into(),
        archived: false,
    }
}

fn request<'a>(repos: &'a Repos, task: &'a Found) -> Request<'a> {
    Request {
        integration: &repos.integration,
        worktree: &repos.worktree,
        branch: BRANCH,
        default_branch: "main",
        task,
    }
}

/// Everything a landing could move: both tips, both HEADs, both trees.
#[derive(Debug, PartialEq)]
struct Snapshot {
    main: String,
    task: String,
    integration_head: String,
    worktree_head: String,
    integration_status: String,
    worktree_status: String,
}

fn snapshot(repos: &Repos) -> Snapshot {
    Snapshot {
        main: git(&repos.integration, &["rev-parse", "main"]),
        task: git(&repos.integration, &["rev-parse", BRANCH]),
        integration_head: git(&repos.integration, &["rev-parse", "--abbrev-ref", "HEAD"]),
        worktree_head: git(&repos.worktree, &["rev-parse", "--abbrev-ref", "HEAD"]),
        integration_status: git(&repos.integration, &["status", "--porcelain"]),
        worktree_status: git(&repos.worktree, &["status", "--porcelain"]),
    }
}

/// A `post-rewrite` hook, shared by both trees, that commits on `main` in the
/// integration tree after a rebase: `times` rebases, then it stops.
fn move_main_after_rebase(repos: &Repos, times: u32) {
    let hooks = PathBuf::from(git(
        &repos.integration,
        &["rev-parse", "--path-format=absolute", "--git-path", "hooks"],
    ));
    fs::create_dir_all(&hooks).unwrap();
    let counter = repos.integration.join(".git").join("moves");
    let integration = repos.integration.to_str().unwrap().replace('\\', "/");
    let counter = counter.to_str().unwrap().replace('\\', "/");
    let script = format!(
        "#!/bin/sh\n\
         unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX\n\
         n=$(cat '{counter}' 2>/dev/null || echo 0)\n\
         [ \"$n\" -ge {times} ] && exit 0\n\
         echo $((n + 1)) > '{counter}'\n\
         git -C '{integration}' commit -q --allow-empty -m \"main moves again $n\"\n"
    );
    let hook = hooks.join("post-rewrite");
    fs::write(&hook, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn a_done_task_lands_and_main_points_at_the_tip_of_its_branch() {
    let repos = repos();
    let task = found("done");
    let before = git(&repos.integration, &["rev-parse", "main"]);

    let landing = land::land(&request(&repos, &task)).unwrap();

    let main = git(&repos.integration, &["rev-parse", "main"]);
    let tip = git(&repos.integration, &["rev-parse", BRANCH]);
    assert_eq!(landing, Landing::Landed { head: tip.clone() });
    assert_eq!(main, tip, "main is the tip of the task branch");
    assert_eq!(git(&repos.integration, &["rev-parse", "main~1"]), before);
    assert_eq!(git(&repos.integration, &["status", "--porcelain"]), "");
    assert!(repos.integration.join("task.txt").exists());
}

#[test]
fn a_task_not_done_is_refused_before_any_write() {
    let repos = repos();
    let task = found("in_progress");
    let before = snapshot(&repos);

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(
        landing,
        Landing::NotDone {
            status: "in_progress".into()
        }
    );
    assert_eq!(snapshot(&repos), before);
}

#[test]
fn a_dirty_worktree_is_refused_before_any_write() {
    let repos = repos();
    fs::write(repos.worktree.join("task.txt"), "uncommitted\n").unwrap();
    let task = found("done");
    let before = snapshot(&repos);

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(landing, Landing::DirtyWorktree);
    assert_eq!(snapshot(&repos), before);
}

#[test]
fn an_integration_tree_off_the_default_branch_is_refused_before_any_write() {
    let repos = repos();
    git(&repos.integration, &["checkout", "-q", "-b", "elsewhere"]);
    let task = found("done");
    let before = snapshot(&repos);

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(
        landing,
        Landing::IntegrationNotOnDefault {
            head: "elsewhere".into()
        }
    );
    assert_eq!(snapshot(&repos), before);
}

#[test]
fn a_dirty_integration_tree_is_refused_before_any_write() {
    let repos = repos();
    fs::write(repos.integration.join("main.txt"), "uncommitted\n").unwrap();
    let task = found("done");
    let before = snapshot(&repos);

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(landing, Landing::DirtyIntegration);
    assert_eq!(snapshot(&repos), before);
}

#[test]
fn a_rebase_conflict_is_aborted_and_the_branch_is_back_where_it_was() {
    let repos = repos();
    commit(
        &repos.integration,
        "shared.txt",
        "main side\n",
        "main edits",
    );
    commit(&repos.worktree, "shared.txt", "task side\n", "task edits");
    let task = found("done");
    let before = snapshot(&repos);

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(landing, Landing::RebaseConflict);
    assert_eq!(snapshot(&repos), before);
    let rebase_merge = git(
        &repos.worktree,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "rebase-merge",
        ],
    );
    assert!(
        !Path::new(&rebase_merge).exists(),
        "no rebase left in progress"
    );
}

#[test]
fn main_moving_between_rebase_and_fast_forward_lands_on_the_second_try() {
    let repos = repos();
    move_main_after_rebase(&repos, 1);
    let task = found("done");

    let landing = land::land(&request(&repos, &task)).unwrap();

    let main = git(&repos.integration, &["rev-parse", "main"]);
    assert_eq!(landing, Landing::Landed { head: main.clone() });
    assert_eq!(main, git(&repos.integration, &["rev-parse", BRANCH]));
    assert_eq!(
        git(&repos.integration, &["log", "-1", "--format=%s", "main~1"]),
        "main moves again 0",
        "the task was rebased onto the commit that moved main"
    );
}

#[test]
fn main_moving_after_each_rebase_is_refused_after_one_retry() {
    let repos = repos();
    move_main_after_rebase(&repos, 2);
    let task = found("done");

    let landing = land::land(&request(&repos, &task)).unwrap();

    assert_eq!(landing, Landing::FastForwardRefused);
    let main = git(&repos.integration, &["rev-parse", "main"]);
    assert_eq!(
        git(&repos.integration, &["log", "-1", "--format=%s", "main"]),
        "main moves again 1",
        "exactly two rebases were tried"
    );
    assert_ne!(main, git(&repos.integration, &["rev-parse", BRANCH]));
    assert_eq!(git(&repos.integration, &["status", "--porcelain"]), "");
    assert_eq!(git(&repos.worktree, &["status", "--porcelain"]), "");
    assert_eq!(
        git(&repos.worktree, &["rev-parse", "--abbrev-ref", "HEAD"]),
        BRANCH
    );
}

/// ADR-ff4569057dc5: git is spawned from src/land alone, and only for the
/// operations it lists.
#[test]
fn only_src_land_runs_git_and_only_the_operations_the_adr_lists() {
    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let land = src.join("land");
    let mut files = Vec::new();
    sources(&src, &mut files);
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        if !file.starts_with(&land) {
            assert!(
                !text.contains("\"git\""),
                "{} spawns git outside src/land",
                file.display()
            );
            continue;
        }
        for forbidden in ["push", "tag", "--force", "--no-ff", "checkout", "reset"] {
            assert!(
                !text.contains(&format!("\"{forbidden}\"")),
                "{} names {forbidden}",
                file.display()
            );
        }
        for call in text.split("(&[").skip(1) {
            let verb = call.split('"').nth(1).unwrap_or_default();
            assert!(
                ["status", "rev-parse", "rebase", "merge"].contains(&verb),
                "{} runs git {verb}",
                file.display()
            );
        }
    }
}
