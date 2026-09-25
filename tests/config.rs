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
        "colour = \"red\"\n[sync]\npoll_seconds = 60\nturbo = true\n[later]\nx = 1\n",
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.sync.poll_seconds, 60);
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
