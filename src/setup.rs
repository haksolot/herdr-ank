//! `herdr-ank setup`: adds to herdr's own `config.toml` what a plugin
//! manifest cannot declare in herdr 0.9.1, the `prefix+a` key for Ouvrir ank
//! and the sidebar rows showing the ank tokens. Once, and only what is
//! missing: a key or a block the user already set wins, and is named on
//! stderr with what to add by hand. The file is backed up before it is
//! written, then `herdr config check` judges it, through `$HERDR_BIN_PATH`
//! (ADR-357c017baf9b); a refusal restores the backup.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use toml::{Table, Value};

/// The key bound to Ouvrir ank.
const KEY: &str = "prefix+a";

const KEY_BLOCK: &str = r#"[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "ank.open"
description = "Ouvrir ank"
"#;

/// The shell form of the same binding, which the README showed first.
const KEY_SHELL_COMMAND: &str = "herdr plugin action invoke open --plugin ank";

const AGENTS_BLOCK: &str = r#"[ui.sidebar.agents]
rows = [
  ["state_icon", "machine", "workspace", "tab"],
  ["agent", "$ank_expires"],
  [{ token = "$ank_title", dim = true }],
]
"#;

const AGENTS_ROWS: &str = r#"  ["agent", "$ank_expires"],
  [{ token = "$ank_title", dim = true }],"#;

const SPACES_BLOCK: &str = r#"[ui.sidebar.spaces]
rows = [
  ["state_icon", "workspace"],
  ["branch", "git_status"],
  ["$ank_queue", "$ank_claims", "$ank_review"],
]
"#;

const SPACES_ROWS: &str = r#"  ["$ank_queue", "$ank_claims", "$ank_review"],"#;

/// What the config already holds for one of the three pieces.
enum Found {
    Missing,
    Present,
    /// Set by the user to something else: what it is, and what to add by hand.
    Taken {
        what: String,
        by_hand: String,
    },
}

pub fn run() -> ExitCode {
    match setup() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("herdr-ank setup: {message}");
            ExitCode::from(1)
        }
    }
}

fn setup() -> Result<(), String> {
    let herdr = match std::env::var_os("HERDR_BIN_PATH") {
        Some(bin) if !bin.is_empty() => PathBuf::from(bin),
        _ => return Err("HERDR_BIN_PATH is not set; run setup from a herdr pane".into()),
    };
    apply(&herdr).map(drop)
}

/// What a setup that succeeded did not write: one line per piece the user
/// already set to something else, each also named on stderr with what to add
/// by hand.
#[derive(Debug, Default)]
pub struct Outcome {
    pub left: Vec<String>,
}

/// The whole of `herdr-ank setup` with `herdr` as herdr's binary; the first
/// daemon of a state dir runs it too (ADR-2df155dab780). An `Err` names what
/// failed, the config being as it was.
pub fn apply(herdr: &Path) -> Result<Outcome, String> {
    let herdr = herdr.to_path_buf();
    let path = config_path(&herdr)?;
    let (text, existed) = match fs::read_to_string(&path) {
        Ok(text) => (text, true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => (String::new(), false),
        Err(err) => return Err(format!("cannot read {}: {err}", path.display())),
    };
    let config: Table = text.parse().map_err(|err| {
        format!(
            "{} is not valid TOML, nothing written: {err}",
            path.display()
        )
    })?;

    let pieces = [
        (key_binding(&config), KEY_BLOCK),
        (sidebar(&config, "agents", AGENTS_ROWS), AGENTS_BLOCK),
        (sidebar(&config, "spaces", SPACES_ROWS), SPACES_BLOCK),
    ];
    let mut added = Vec::new();
    let mut outcome = Outcome::default();
    for (found, block) in pieces {
        match found {
            Found::Missing => added.push(block),
            Found::Present => {}
            Found::Taken { what, by_hand } => {
                eprintln!(
                    "herdr-ank setup: {what} in {}; left intact. To add by hand:\n{by_hand}",
                    path.display()
                );
                outcome.left.push(what);
            }
        }
    }
    if added.is_empty() {
        println!("herdr-ank setup: nothing to add to {}", path.display());
        return Ok(outcome);
    }

    let backup = if existed {
        let backup = backup_path(&path);
        fs::copy(&path, &backup)
            .map_err(|err| format!("cannot back up {}: {err}", path.display()))?;
        Some(backup)
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("cannot create {}: {err}", parent.display()))?;
        }
        None
    };
    let mut new_text = text.clone();
    if !new_text.is_empty() && !new_text.ends_with('\n') {
        new_text.push('\n');
    }
    for block in &added {
        if !new_text.is_empty() {
            new_text.push('\n');
        }
        new_text.push_str(block);
    }
    fs::write(&path, new_text).map_err(|err| format!("cannot write {}: {err}", path.display()))?;

    let check = Command::new(&herdr)
        .args(["config", "check"])
        .env("HERDR_CONFIG_PATH", &path)
        .output()
        .map_err(|err| format!("cannot run {} config check: {err}", herdr.display()))?;
    if !check.status.success() {
        let restored = match &backup {
            Some(backup) => fs::copy(backup, &path).map(drop),
            None => fs::remove_file(&path),
        };
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        return Err(match restored {
            Ok(()) => format!(
                "herdr config check refused {}; restored as it was:\n{diagnostics}",
                path.display()
            ),
            Err(err) => format!(
                "herdr config check refused {} and restoring it failed ({err}); the backup is {}:\n{diagnostics}",
                path.display(),
                backup.as_deref().map_or("absent".into(), |b| b.display().to_string())
            ),
        });
    }

    for block in &added {
        let header = block.lines().next().unwrap_or_default();
        println!("herdr-ank setup: added {header} to {}", path.display());
    }
    if let Some(backup) = &backup {
        println!("herdr-ank setup: backup in {}", backup.display());
    }
    match Command::new(&herdr)
        .args(["server", "reload-config"])
        .output()
    {
        Ok(reload) if reload.status.success() => {}
        Ok(reload) => eprintln!(
            "herdr-ank setup: herdr server reload-config failed; herdr reads the file at its next start: {}",
            String::from_utf8_lossy(&reload.stderr).trim()
        ),
        Err(err) => eprintln!(
            "herdr-ank setup: cannot run herdr server reload-config ({err}); herdr reads the file at its next start"
        ),
    }
    Ok(outcome)
}

/// `$HERDR_CONFIG_PATH`, which herdr honours, else the `Config:` line of
/// `herdr --help`, the path herdr documents on this system.
fn config_path(herdr: &Path) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("HERDR_CONFIG_PATH").filter(|p| !p.is_empty()) {
        return Ok(path.into());
    }
    let help = Command::new(herdr)
        .arg("--help")
        .output()
        .map_err(|err| format!("cannot run {} --help: {err}", herdr.display()))?;
    String::from_utf8_lossy(&help.stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("Config:"))
        .map(|path| PathBuf::from(path.trim()))
        .ok_or_else(|| {
            format!(
                "{} --help names no Config: path; set HERDR_CONFIG_PATH",
                herdr.display()
            )
        })
}

/// What `prefix+a` is bound to: a `[[keys.command]]`, or a built-in action
/// of `[keys]`.
fn key_binding(config: &Table) -> Found {
    let taken = |what: String| Found::Taken {
        what,
        by_hand: format!("{KEY_BLOCK}(with another key than {KEY} if you keep yours)"),
    };
    let Some(keys) = config.get("keys").and_then(Value::as_table) else {
        return Found::Missing;
    };
    for command in keys
        .get("command")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_table)
    {
        if !command.get("key").is_some_and(binds_key) {
            continue;
        }
        let runs = command.get("command").and_then(Value::as_str).unwrap_or("");
        let plugin_action = command.get("type").and_then(Value::as_str) == Some("plugin_action");
        if (plugin_action && runs == "ank.open") || runs.trim() == KEY_SHELL_COMMAND {
            return Found::Present;
        }
        return taken(format!("{KEY} already runs `{runs}` ([[keys.command]])"));
    }
    for (action, binding) in keys {
        if binds_key(binding) {
            return taken(format!("{KEY} is already bound to keys.{action}"));
        }
    }
    Found::Missing
}

/// A binding is one key or a list of them (`["prefix+h", "alt+enter"]`).
fn binds_key(binding: &Value) -> bool {
    match binding {
        Value::String(key) => key.trim().eq_ignore_ascii_case(KEY),
        Value::Array(keys) => keys.iter().any(binds_key),
        _ => false,
    }
}

/// `[ui.sidebar.<section>]`: missing, already showing ank tokens, or the
/// user's own layout.
fn sidebar(config: &Table, section: &str, rows: &str) -> Found {
    let block = config
        .get("ui")
        .and_then(|ui| ui.get("sidebar"))
        .and_then(|sidebar| sidebar.get(section));
    match block {
        None => Found::Missing,
        Some(block) if names_ank_token(block) => Found::Present,
        Some(_) => Found::Taken {
            what: format!("[ui.sidebar.{section}] is already set"),
            by_hand: format!("add to its rows:\n{rows}"),
        },
    }
}

fn names_ank_token(value: &Value) -> bool {
    match value {
        Value::String(s) => s.starts_with("$ank_"),
        Value::Array(items) => items.iter().any(names_ank_token),
        Value::Table(table) => table.values().any(names_ank_token),
        _ => false,
    }
}

/// `config.toml.bak-ank-<UTC YYYYMMDDHHMMSS>` next to the config, suffixed
/// `-<n>` should that name already exist.
fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map_or("config.toml".into(), |n| n.to_string_lossy().into_owned());
    let base = path.with_file_name(format!("{name}.bak-ank-{}", utc_stamp(SystemTime::now())));
    let mut candidate = base.clone();
    let mut n = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{}-{n}", base.display()));
        n += 1;
    }
    candidate
}

fn utc_stamp(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Howard Hinnant's civil_from_days, on the proleptic Gregorian calendar.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}{:02}{:02}{:02}",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_stamp_is_utc_calendar_time() {
        let at = UNIX_EPOCH + Duration::from_secs(1_790_416_989);
        assert_eq!(utc_stamp(at), "20260926100309");
        assert_eq!(utc_stamp(UNIX_EPOCH), "19700101000000");
        let leap = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(utc_stamp(leap), "20000229000000");
    }
}
