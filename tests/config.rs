use std::fs;

use herdr_ank::config::Config;

#[test]
fn missing_file_gives_the_documented_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.agent.kind, "claude");
    assert!(config.agent.args.is_empty());
    assert_eq!(config.sync.poll_seconds, 30);
    assert!(config.notify.done);
    assert_eq!(config.notify.expiring_minutes, 10);
    assert!(config.notify.review);
}

#[test]
fn partial_file_overrides_what_it_names_and_keeps_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.toml"),
        "[agent]\nargs = [\"--model\", \"opus\"]\n\n[notify]\nexpiring_minutes = 5\n",
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.agent.kind, "claude");
    assert_eq!(config.agent.args, ["--model", "opus"]);
    assert_eq!(config.sync.poll_seconds, 30);
    assert!(config.notify.done);
    assert_eq!(config.notify.expiring_minutes, 5);
    assert!(config.notify.review);
}

#[test]
fn unknown_keys_and_tables_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.toml"),
        "colour = \"red\"\n[sync]\npoll_seconds = 20\nturbo = true\n[later]\nx = 1\n",
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.sync.poll_seconds, 20);
    assert_eq!(config.agent.kind, "claude");
}

#[test]
fn a_mistyped_key_is_an_error_that_names_it() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.toml"),
        "[sync]\npoll_seconds = \"thirty\"\n",
    )
    .unwrap();

    let err = Config::load(dir.path()).unwrap_err().to_string();

    assert!(err.contains("sync.poll_seconds"), "{err}");
}

#[test]
fn a_mistyped_array_element_or_table_is_named_too() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.toml"),
        "[agent]\nargs = [\"-v\", 3]\n",
    )
    .unwrap();
    let err = Config::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("agent.args"), "{err}");

    fs::write(dir.path().join("config.toml"), "notify = true\n").unwrap();
    let err = Config::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("notify"), "{err}");

    fs::write(
        dir.path().join("config.toml"),
        "[notify]\nexpiring_minutes = -1\n",
    )
    .unwrap();
    let err = Config::load(dir.path()).unwrap_err().to_string();
    assert!(err.contains("notify.expiring_minutes"), "{err}");
}

fn load_poll_seconds(value: &str) -> Result<Config, String> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.toml"),
        format!("[sync]\npoll_seconds = {value}\n"),
    )
    .unwrap();
    Config::load(dir.path()).map_err(|e| e.to_string())
}

#[test]
fn poll_seconds_zero_is_refused_naming_key_and_bound() {
    let err = load_poll_seconds("0").unwrap_err();
    assert!(err.contains("sync.poll_seconds"), "{err}");
    assert!(err.contains("1") && err.contains("30"), "{err}");
}

#[test]
fn poll_seconds_thirty_is_the_upper_bound_and_accepted() {
    assert_eq!(load_poll_seconds("30").unwrap().sync.poll_seconds, 30);
    assert_eq!(load_poll_seconds("1").unwrap().sync.poll_seconds, 1);
}

#[test]
fn poll_seconds_thirty_one_is_refused_naming_key_and_bound() {
    let err = load_poll_seconds("31").unwrap_err();
    assert!(err.contains("sync.poll_seconds"), "{err}");
    assert!(err.contains("30"), "{err}");
}

fn load(text: &str) -> Result<Config, String> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("config.toml"), text).unwrap();
    Config::load(dir.path()).map_err(|e| e.to_string())
}

#[test]
fn land_cleanup_steps_all_default_to_true() {
    let dir = tempfile::tempdir().unwrap();
    let missing_file = Config::load(dir.path()).unwrap();
    let no_land_table = load("[sync]\npoll_seconds = 20\n").unwrap();
    let empty_land_table = load("[land]\n").unwrap();

    for config in [missing_file, no_land_table, empty_land_table] {
        assert!(config.land.close_tab);
        assert!(config.land.remove_worktree);
        assert!(config.land.delete_branch);
        assert!(config.land.delete_remote_branch);
    }
}

#[test]
fn each_land_step_reads_false_and_leaves_the_others_true() {
    let config = load("[land]\nclose_tab = false\n").unwrap();
    assert!(!config.land.close_tab);
    assert!(config.land.remove_worktree && config.land.delete_branch);
    assert!(config.land.delete_remote_branch);

    let config = load("[land]\nremove_worktree = false\n").unwrap();
    assert!(!config.land.remove_worktree);
    assert!(config.land.close_tab && config.land.delete_branch);
    assert!(config.land.delete_remote_branch);

    let config = load("[land]\ndelete_branch = false\n").unwrap();
    assert!(!config.land.delete_branch);
    assert!(config.land.close_tab && config.land.remove_worktree);
    assert!(config.land.delete_remote_branch);

    let config = load("[land]\ndelete_remote_branch = false\n").unwrap();
    assert!(!config.land.delete_remote_branch);
    assert!(config.land.close_tab && config.land.remove_worktree);
    assert!(config.land.delete_branch);
}

#[test]
fn a_mistyped_land_key_is_an_error_that_names_it() {
    for key in [
        "close_tab",
        "remove_worktree",
        "delete_branch",
        "delete_remote_branch",
    ] {
        let err = load(&format!("[land]\n{key} = \"no\"\n")).unwrap_err();
        assert!(err.contains(&format!("land.{key}")), "{err}");
    }
    let err = load("land = true\n").unwrap_err();
    assert!(err.contains("land"), "{err}");
}

#[test]
fn an_unknown_key_under_land_is_ignored() {
    let config = load("[land]\npush_main = true\nclose_tab = false\n").unwrap();
    assert!(!config.land.close_tab);
    assert!(config.land.delete_branch);
}
