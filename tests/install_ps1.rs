//! `install.ps1`, the manifest's `[[build]]` on Windows, run by Windows
//! PowerShell 5.1 against a release held in a local directory
//! (`HERDR_ANK_RELEASE_BASE`) and under a PATH that holds the system and no
//! `cargo`.
#![cfg(windows)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TARGET: &str = "x86_64-pc-windows-msvc";
const BINARY: &[u8] = b"MZ herdr-ank from the release";

fn version() -> String {
    let manifest: toml::Table = fs::read_to_string(Path::new(ROOT).join("herdr-plugin.toml"))
        .unwrap()
        .parse()
        .unwrap();
    manifest["version"].as_str().unwrap().to_string()
}

fn system_root() -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
}

fn powershell() -> PathBuf {
    system_root().join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

/// Runs a PowerShell snippet for the test's own setup, and asserts it succeeds:
/// any error stops it, since `-Command` exits 0 when only an earlier statement
/// failed.
fn ps(script: &str) {
    let script = format!("$ErrorActionPreference = 'Stop'; {script}");
    let out = Command::new(powershell())
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{script}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Case {
    _tmp: tempfile::TempDir,
    plugin: PathBuf,
    release: PathBuf,
}

impl Case {
    /// A plugin checkout holding the manifest and install.ps1, and an empty
    /// release directory.
    fn new() -> Case {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("plugin");
        let release = tmp.path().join("release");
        for dir in [&plugin, &release] {
            fs::create_dir(dir).unwrap();
        }
        for file in ["herdr-plugin.toml", "install.ps1"] {
            fs::copy(Path::new(ROOT).join(file), plugin.join(file)).unwrap();
        }
        Case {
            _tmp: tmp,
            plugin,
            release,
        }
    }

    fn asset(&self) -> String {
        format!("herdr-ank-{}-{TARGET}.zip", version())
    }

    /// Publishes the archive the release workflow builds: herdr-ank.exe at
    /// the root of a zip.
    fn publish_archive(&self) {
        let staging = self._tmp.path().join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("herdr-ank.exe"), BINARY).unwrap();
        ps(&format!(
            "Add-Type -AssemblyName System.IO.Compression.FileSystem; \
             [IO.Compression.ZipFile]::CreateFromDirectory('{}', '{}')",
            staging.display(),
            self.release.join(self.asset()).display()
        ));
    }

    /// Writes SHA256SUMS the way sha256sum does, with the archive's true sum,
    /// computed by .NET rather than by the Get-FileHash under test.
    fn publish_sums(&self) {
        ps(&format!(
            "$sha = [Security.Cryptography.SHA256]::Create(); \
             $bytes = $sha.ComputeHash([IO.File]::ReadAllBytes('{}')); \
             $h = ([BitConverter]::ToString($bytes) -replace '-', '').ToLower(); \
             [IO.File]::WriteAllText('{}', $h + '  {}' + [char]10)",
            self.release.join(self.asset()).display(),
            self.release.join("SHA256SUMS").display(),
            self.asset()
        ));
    }

    fn publish_wrong_sums(&self) {
        fs::write(
            self.release.join("SHA256SUMS"),
            format!("{}  {}\n", "0".repeat(64), self.asset()),
        )
        .unwrap();
    }

    /// install.ps1 as herdr's [[build]] runs it, from another directory, with
    /// a PATH that holds the Windows system directories and no cargo.
    fn install(&self) -> Output {
        let system = system_root();
        let path = std::env::join_paths([
            system.join("System32"),
            system.clone(),
            system.join(r"System32\WindowsPowerShell\v1.0"),
        ])
        .unwrap();
        Command::new(powershell())
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "install.ps1",
            ])
            .current_dir(&self.plugin)
            .env("PATH", path)
            .env("HERDR_ANK_RELEASE_BASE", &self.release)
            .output()
            .unwrap()
    }

    fn installed(&self) -> PathBuf {
        self.plugin.join(r"bin\herdr-ank.exe")
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_valid_archive_puts_herdr_ank_exe_in_bin() {
    let case = Case::new();
    case.publish_archive();
    case.publish_sums();

    let out = case.install();

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(fs::read(case.installed()).unwrap(), BINARY);
}

#[test]
fn a_wrong_sum_installs_nothing_and_exits_1_without_cargo() {
    let case = Case::new();
    case.publish_archive();
    case.publish_wrong_sums();

    let out = case.install();

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(!case.installed().exists());
    let err = stderr(&out);
    assert!(err.contains("does not match its sum"), "{err}");
    assert!(err.contains("cargo is not on PATH"), "{err}");
}

#[test]
fn a_missing_archive_without_cargo_exits_1_naming_the_url() {
    let case = Case::new();

    let out = case.install();

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(!case.installed().exists());
    let err = stderr(&out);
    let url = format!("{}/{}", case.release.display(), case.asset());
    assert!(err.contains(&url), "{url} not in: {err}");
    assert!(err.contains("cargo is not on PATH"), "{err}");
}
