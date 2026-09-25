//! `herdr-ank tui`: find the corpus of the workspace herdr opened the pane
//! from, then become `ank tui --repo <it>`.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;

/// The fields of herdr's `PluginInvocationContext` this pane reads; the
/// others are ignored.
#[derive(Deserialize)]
struct Context {
    workspace_cwd: Option<PathBuf>,
    worktree: Option<Worktree>,
}

#[derive(Deserialize)]
struct Worktree {
    checkout_path: PathBuf,
}

/// The directory the corpus search starts from: the current worktree's
/// checkout, else the workspace cwd. Never the process cwd, which herdr sets
/// to the plugin root.
pub fn start_dir(context_json: &str) -> Option<PathBuf> {
    let context: Context = serde_json::from_str(context_json).ok()?;
    context
        .worktree
        .map(|w| w.checkout_path)
        .or(context.workspace_cwd)
}

/// The first directory, from `start` upwards, that holds a `.ank/`.
pub fn find_corpus(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(".ank").is_dir())
        .map(Path::to_path_buf)
}

/// Replaces this process with `ank tui --repo <corpus>`. Returns only on
/// failure, after saying why on stderr, so herdr closes the pane at once.
pub fn run() -> ExitCode {
    let context = std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default();
    let Some(start) = start_dir(&context) else {
        eprintln!(
            "herdr-ank tui: HERDR_PLUGIN_CONTEXT_JSON names no worktree or workspace cwd; open this pane from herdr"
        );
        return ExitCode::from(1);
    };
    let Some(repo) = find_corpus(&start) else {
        eprintln!("{}", not_found(&start));
        return ExitCode::from(1);
    };
    let err = Command::new("ank")
        .arg("tui")
        .arg("--repo")
        .arg(&repo)
        .exec();
    eprintln!(
        "herdr-ank tui: cannot run `ank tui --repo {}`: {err}",
        repo.display()
    );
    ExitCode::from(1)
}

fn not_found(start: &Path) -> String {
    format!(
        "herdr-ank tui: no .ank/ in {} or any of its parents\nnext: ank init",
        start.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn worktree_checkout_path_wins_over_workspace_cwd() {
        let json = r#"{"workspace_cwd":"/w","worktree":{"repo_key":"k","repo_name":"n",
            "repo_root":"/r","checkout_path":"/r/wt","is_linked_worktree":true},"tab_id":"t"}"#;
        assert_eq!(start_dir(json), Some(PathBuf::from("/r/wt")));
    }

    #[test]
    fn workspace_cwd_when_there_is_no_worktree() {
        let json = r#"{"workspace_id":"1","workspace_cwd":"/home/me/repo","later_field":3}"#;
        assert_eq!(start_dir(json), Some(PathBuf::from("/home/me/repo")));
    }

    #[test]
    fn no_start_dir_without_either_field_or_with_bad_json() {
        assert_eq!(start_dir(r#"{"focused_pane_cwd":"/x"}"#), None);
        assert_eq!(start_dir("not json"), None);
    }

    #[test]
    fn corpus_is_the_nearest_ancestor_holding_ank_dir() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let deep = repo.join("src/deep");
        fs::create_dir_all(repo.join(".ank")).unwrap();
        fs::create_dir_all(&deep).unwrap();

        assert_eq!(find_corpus(&deep), Some(repo.clone()));
        assert_eq!(find_corpus(&repo), Some(repo));
    }

    #[test]
    fn no_corpus_when_no_ancestor_holds_ank_dir() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("a/b");
        fs::create_dir_all(&dir).unwrap();
        // a plain file named .ank is not a corpus
        fs::write(root.path().join("a/.ank"), "").unwrap();

        assert_eq!(find_corpus(&dir), None);
    }

    #[test]
    fn not_found_names_the_searched_path_and_ank_init() {
        let message = not_found(Path::new("/home/me/elsewhere"));
        assert!(message.contains("/home/me/elsewhere"), "{message}");
        assert!(message.contains("ank init"), "{message}");
    }
}
