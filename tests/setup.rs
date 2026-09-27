//! `herdr-ank setup` adds the `prefix+a` key and the sidebar rows to herdr's
//! `config.toml` once, never over what the user set, and hands the result to
//! `herdr config check` then `herdr server reload-config` (ADR-357c017baf9b).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use toml::{Table, Value};

mod support;

fn scratch(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("setup-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::create_dir_all(dir.join("herdr")).unwrap();
    dir
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("herdr").join("config.toml")
}

/// A fake herdr answering `config check` as `check` says.
fn fake_herdr(dir: &Path, check: (&str, u8)) {
    let bin = dir.join("bin");
    support::link(&bin, "herdr");
    support::write_answers(
        &bin,
        &[
            support::answer(&["config", "check"], check.0, "", check.1),
            support::answer(&["server", "reload-config"], "", "", 0),
        ],
    );
}

fn setup_command(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_herdr-ank"));
    command
        .arg("setup")
        .env(
            "HERDR_BIN_PATH",
            dir.join("bin")
                .join(format!("herdr{}", std::env::consts::EXE_SUFFIX)),
        )
        .env("HERDR_SOCKET_PATH", dir.join("herdr.sock"))
        .env("HERDR_CONFIG_PATH", config_path(dir));
    command
}

fn setup(dir: &Path) -> Output {
    setup_command(dir).output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir.join("herdr"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("config.toml.bak-ank-")
        })
        .collect();
    found.sort();
    found
}

fn argvs(dir: &Path) -> Vec<Vec<String>> {
    support::calls(&dir.join("bin"))
        .into_iter()
        .map(|call| call.argv)
        .collect()
}

fn parsed(dir: &Path) -> Table {
    fs::read_to_string(config_path(dir))
        .unwrap()
        .parse()
        .unwrap()
}

fn toml_value(text: &str) -> Value {
    format!("v = {text}").parse::<Table>().unwrap()["v"].clone()
}

fn key_commands(config: &Table) -> Vec<Table> {
    config
        .get("keys")
        .and_then(|keys| keys.get("command"))
        .and_then(Value::as_array)
        .map(|commands| {
            commands
                .iter()
                .map(|c| c.as_table().unwrap().clone())
                .collect()
        })
        .unwrap_or_default()
}

fn sidebar_rows(config: &Table, section: &str) -> Value {
    config["ui"]["sidebar"][section]["rows"].clone()
}

const AGENT_ROWS: &str = r#"[
  ["state_icon", "machine", "workspace", "tab"],
  ["agent", "$ank_expires"],
  [{ token = "$ank_title", dim = true }],
]"#;

const SPACE_ROWS: &str = r#"[
  ["state_icon", "workspace"],
  ["branch", "git_status"],
  ["$ank_queue", "$ank_claims", "$ank_review"],
]"#;

const MINE: &str = "# my herdr\n[ui]\nsidebar_width = 30\n";

fn assert_ank_open_bound(config: &Table) {
    let bound: Vec<Table> = key_commands(config)
        .into_iter()
        .filter(|c| c["key"].as_str() == Some("prefix+a"))
        .collect();
    assert_eq!(bound.len(), 1, "{bound:?}");
    assert_eq!(bound[0]["type"].as_str(), Some("plugin_action"));
    assert_eq!(bound[0]["command"].as_str(), Some("ank.open"));
}

#[test]
fn a_blank_config_gets_the_key_and_both_sidebars_after_a_backup_then_is_checked_and_reloaded() {
    let dir = scratch("blank");
    fake_herdr(&dir, ("config: ok\n", 0));
    fs::write(config_path(&dir), MINE).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = fs::read_to_string(config_path(&dir)).unwrap();
    assert!(
        text.starts_with(MINE),
        "the user's lines come first:\n{text}"
    );
    let config = parsed(&dir);
    assert_eq!(config["ui"]["sidebar_width"].as_integer(), Some(30));
    assert_ank_open_bound(&config);
    assert_eq!(sidebar_rows(&config, "agents"), toml_value(AGENT_ROWS));
    assert_eq!(sidebar_rows(&config, "spaces"), toml_value(SPACE_ROWS));

    let backups = backups(&dir);
    assert_eq!(backups.len(), 1, "{backups:?}");
    let name = backups[0]
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let stamp = name.trim_start_matches("config.toml.bak-ank-");
    assert!(
        stamp.len() == 14 && stamp.chars().all(|c| c.is_ascii_digit()),
        "a UTC timestamp, YYYYMMDDHHMMSS: {name}"
    );
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), MINE);

    assert_eq!(
        argvs(&dir),
        [vec!["config", "check"], vec!["server", "reload-config"]]
    );
}

#[test]
fn a_missing_config_is_created_without_a_backup() {
    let dir = scratch("missing");
    fake_herdr(&dir, ("config: ok\n", 0));

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    assert_ank_open_bound(&config);
    assert_eq!(sidebar_rows(&config, "agents"), toml_value(AGENT_ROWS));
    assert_eq!(sidebar_rows(&config, "spaces"), toml_value(SPACE_ROWS));
    assert!(backups(&dir).is_empty());
}

#[test]
fn run_again_it_writes_nothing_and_makes_no_backup() {
    let dir = scratch("again");
    fake_herdr(&dir, ("config: ok\n", 0));
    fs::write(config_path(&dir), MINE).unwrap();
    assert!(setup(&dir).status.success());
    let first = fs::read(config_path(&dir)).unwrap();
    let backups_before = backups(&dir);
    let calls_before = argvs(&dir).len();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(fs::read(config_path(&dir)).unwrap(), first);
    assert_eq!(backups(&dir), backups_before);
    assert_eq!(argvs(&dir).len(), calls_before, "{:?}", argvs(&dir));
    assert!(
        !stderr(&output).contains("by hand"),
        "nothing to do by hand: {}",
        stderr(&output)
    );
}

#[test]
fn the_readme_shell_binding_counts_as_already_set_up() {
    let dir = scratch("shell-binding");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine = "[[keys.command]]\nkey = \"prefix+a\"\ncommand = \"herdr plugin action invoke open --plugin ank\"\ndescription = \"Ouvrir ank\"\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    let bound: Vec<Table> = key_commands(&config)
        .into_iter()
        .filter(|c| c["key"].as_str() == Some("prefix+a"))
        .collect();
    assert_eq!(bound.len(), 1, "{bound:?}");
    assert!(!stderr(&output).contains("prefix+a"), "{}", stderr(&output));
}

#[test]
fn prefix_a_bound_to_something_else_is_left_intact_and_named_with_the_line_to_add() {
    let dir = scratch("conflict");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine = "[[keys.command]]\nkey = \"prefix+a\"\ncommand = \"lazygit\"\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    let commands = key_commands(&config);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0]["command"].as_str(), Some("lazygit"));
    assert_eq!(sidebar_rows(&config, "agents"), toml_value(AGENT_ROWS));
    assert_eq!(sidebar_rows(&config, "spaces"), toml_value(SPACE_ROWS));
    let err = stderr(&output);
    assert!(err.contains("prefix+a"), "{err}");
    assert!(err.contains(r#"command = "ank.open""#), "{err}");
}

#[test]
fn prefix_a_bound_to_a_builtin_action_is_a_conflict_too() {
    let dir = scratch("builtin");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine = "[keys]\ntoggle_sidebar = \"prefix+a\"\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    assert!(key_commands(&config).is_empty());
    assert_eq!(config["keys"]["toggle_sidebar"].as_str(), Some("prefix+a"));
    let err = stderr(&output);
    assert!(err.contains("toggle_sidebar"), "{err}");
    assert!(err.contains(r#"command = "ank.open""#), "{err}");
}

#[test]
fn an_existing_sidebar_is_left_intact_and_named_with_the_rows_to_add() {
    let dir = scratch("sidebar");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine = "[ui.sidebar.agents]\nrows = [[\"agent\"]]\n\n[ui.sidebar.spaces]\nrows = [[\"workspace\"]]\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    assert_eq!(
        sidebar_rows(&config, "agents"),
        toml_value(r#"[["agent"]]"#)
    );
    assert_eq!(
        sidebar_rows(&config, "spaces"),
        toml_value(r#"[["workspace"]]"#)
    );
    assert_ank_open_bound(&config);
    let err = stderr(&output);
    assert!(err.contains("[ui.sidebar.agents]"), "{err}");
    assert!(err.contains(r#"["agent", "$ank_expires"]"#), "{err}");
    assert!(
        err.contains(r#"[{ token = "$ank_title", dim = true }]"#),
        "{err}"
    );
    assert!(err.contains("[ui.sidebar.spaces]"), "{err}");
    assert!(
        err.contains(r#"["$ank_queue", "$ank_claims", "$ank_review"]"#),
        "{err}"
    );
}

#[test]
fn a_failed_check_restores_the_backup_and_exits_1() {
    let dir = scratch("check-fails");
    fake_herdr(&dir, ("config: issues found\nunknown sidebar token\n", 1));
    fs::write(config_path(&dir), MINE).unwrap();

    let output = setup(&dir);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(fs::read_to_string(config_path(&dir)).unwrap(), MINE);
    assert!(
        stderr(&output).contains("unknown sidebar token"),
        "{}",
        stderr(&output)
    );
    assert_eq!(argvs(&dir), [vec!["config", "check"]]);
}

#[test]
fn without_herdr_config_path_it_writes_where_herdr_help_says() {
    let dir = scratch("help-path");
    let bin = dir.join("bin");
    support::link(&bin, "herdr");
    let help = format!(
        "herdr — terminal workspace manager\n\nConfig: {}\nLogs:   elsewhere\n",
        config_path(&dir).display()
    );
    support::write_answers(&bin, &[support::answer(&["--help"], &help, "", 0)]);

    let output = setup_command(&dir)
        .env_remove("HERDR_CONFIG_PATH")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_ank_open_bound(&parsed(&dir));
}

#[test]
fn a_custom_prefix_gets_the_same_prefix_relative_key() {
    let dir = scratch("custom-prefix");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine =
        "[keys]\nprefix = \"ctrl+space\"\nsplit_horizontal = [\"prefix+h\", \"alt+enter\"]\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    assert_eq!(config["keys"]["prefix"].as_str(), Some("ctrl+space"));
    assert_ank_open_bound(&config);
    let text = fs::read_to_string(config_path(&dir)).unwrap();
    assert!(!text.contains("ctrl+space+a"), "{text}");
    assert!(!text.contains("ctrl+b"), "{text}");
}

#[test]
fn prefix_a_among_the_keys_of_a_builtin_action_is_a_conflict() {
    let dir = scratch("builtin-list");
    fake_herdr(&dir, ("config: ok\n", 0));
    let mine = "[keys]\nprefix = \"ctrl+space\"\ntoggle_sidebar = [\"alt+s\", \"prefix+a\"]\n";
    fs::write(config_path(&dir), mine).unwrap();

    let output = setup(&dir);

    assert!(output.status.success(), "{}", stderr(&output));
    let config = parsed(&dir);
    assert!(
        key_commands(&config).is_empty(),
        "{:?}",
        key_commands(&config)
    );
    let err = stderr(&output);
    assert!(err.contains("toggle_sidebar"), "{err}");
    assert!(err.contains(r#"command = "ank.open""#), "{err}");
}
