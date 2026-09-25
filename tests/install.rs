//! `install.sh`, the manifest's `[[build]]`, run against a `file://` release
//! in a tempdir and under a PATH built for the test: the tools the script needs,
//! and `cargo` only when a case asks for it.
//!
//! install.sh is POSIX sh, so these tests run where it runs (ADR-599b6f424271).

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TOOLS: &[&str] = &[
    "sh",
    "sed",
    "awk",
    "curl",
    "sha256sum",
    "tar",
    "gzip",
    "mkdir",
    "chmod",
    "cp",
    "rm",
    "mktemp",
    "dirname",
];

fn manifest() -> toml::Table {
    fs::read_to_string(Path::new(ROOT).join("herdr-plugin.toml"))
        .unwrap()
        .parse()
        .unwrap()
}

fn version() -> String {
    manifest()["version"].as_str().unwrap().to_string()
}

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        other => panic!("no release target for {other:?}"),
    }
}

fn which(tool: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(tool))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{tool} is not on PATH"))
}

fn write_exec(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

struct Case {
    _tmp: tempfile::TempDir,
    plugin: PathBuf,
    release: PathBuf,
    path: PathBuf,
}

impl Case {
    /// A plugin checkout holding the manifest and install.sh, an empty release
    /// directory, and a PATH with the tools the script needs and no cargo.
    fn new() -> Case {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("plugin");
        let release = tmp.path().join("release");
        let path = tmp.path().join("path");
        for dir in [&plugin, &release, &path] {
            fs::create_dir(dir).unwrap();
        }
        for file in ["herdr-plugin.toml", "install.sh"] {
            fs::copy(Path::new(ROOT).join(file), plugin.join(file)).unwrap();
        }
        for tool in TOOLS.iter().chain(["uname"].iter()) {
            symlink(which(tool), path.join(tool)).unwrap();
        }
        Case {
            _tmp: tmp,
            plugin,
            release,
            path,
        }
    }

    fn asset(&self) -> String {
        format!("herdr-ank-{}-{}.tar.gz", version(), target())
    }

    /// Publishes an archive holding `herdr-ank` at its root, and its sum.
    fn publish(&self) {
        let staging = self.release.join("staging");
        fs::create_dir(&staging).unwrap();
        write_exec(&staging.join("herdr-ank"), "#!/bin/sh\necho released\n");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(self.release.join(self.asset()))
            .arg("-C")
            .arg(&staging)
            .arg("herdr-ank")
            .status()
            .unwrap();
        assert!(status.success());
        let sum = Command::new("sha256sum")
            .arg(self.asset())
            .current_dir(&self.release)
            .output()
            .unwrap();
        assert!(sum.status.success());
        fs::write(self.release.join("SHA256SUMS"), sum.stdout).unwrap();
    }

    fn corrupt_sums(&self) {
        let line = format!("{}  {}\n", "0".repeat(64), self.asset());
        fs::write(self.release.join("SHA256SUMS"), line).unwrap();
    }

    /// A cargo whose `build --release` leaves a binary where cargo would.
    fn with_cargo(&self) {
        write_exec(
            &self.path.join("cargo"),
            "#!/bin/sh\n[ \"$*\" = 'build --release' ] || exit 2\n\
             mkdir -p target/release\n\
             printf '#!/bin/sh\\necho built\\n' > target/release/herdr-ank\n\
             chmod +x target/release/herdr-ank\n",
        );
    }

    fn with_uname(&self, sm: &str) {
        let fake = self.path.join("uname");
        fs::remove_file(&fake).unwrap();
        write_exec(&fake, &format!("#!/bin/sh\necho '{sm}'\n"));
    }

    fn base(&self) -> String {
        format!("file://{}", self.release.display())
    }

    fn install(&self) -> Output {
        Command::new(which("sh"))
            .arg("install.sh")
            .current_dir(&self.plugin)
            .env_clear()
            .env("PATH", &self.path)
            .env("HERDR_ANK_RELEASE_BASE", self.base())
            .output()
            .unwrap()
    }

    fn binary(&self) -> PathBuf {
        self.plugin.join("bin").join("herdr-ank")
    }

    fn run_binary(&self) -> String {
        let out = Command::new(self.binary()).output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_valid_release_archive_lands_executable_in_bin() {
    let case = Case::new();
    case.publish();
    let out = case.install();
    assert!(out.status.success(), "{}", stderr(&out));
    let mode = fs::metadata(case.binary()).unwrap().permissions().mode();
    assert!(mode & 0o111 != 0, "{mode:o}");
    assert_eq!(case.run_binary(), "released\n");
}

#[test]
fn a_wrong_sum_falls_back_to_cargo_when_cargo_is_on_path() {
    let case = Case::new();
    case.publish();
    case.corrupt_sums();
    case.with_cargo();
    let out = case.install();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(case.run_binary(), "built\n");
}

#[test]
fn a_wrong_sum_without_cargo_exits_1_naming_the_url_and_cargo() {
    let case = Case::new();
    case.publish();
    case.corrupt_sums();
    let out = case.install();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains(&format!("{}/{}", case.base(), case.asset())),
        "{err}"
    );
    assert!(err.contains("cargo"), "{err}");
    assert!(!case.binary().exists());
}

#[test]
fn a_missing_release_without_cargo_exits_1_naming_the_url_and_cargo() {
    let case = Case::new();
    let out = case.install();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains(&format!("{}/{}", case.base(), case.asset())),
        "{err}"
    );
    assert!(err.contains("cargo"), "{err}");
}

#[test]
fn an_unknown_target_is_named() {
    let case = Case::new();
    case.publish();
    case.with_uname("Linux riscv64");
    let out = case.install();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("Linux riscv64"), "{err}");
    assert!(err.contains("cargo"), "{err}");
}

#[test]
fn the_manifest_builds_with_install_sh_and_runs_bin_herdr_ank() {
    let manifest = manifest();
    assert_eq!(
        manifest["platforms"],
        toml::Value::try_from(["linux", "macos", "windows"]).unwrap()
    );
    let build = manifest["build"].as_array().unwrap();
    assert_eq!(build.len(), 2);
    assert_eq!(
        build[0]["command"],
        toml::Value::try_from(["sh", "install.sh"]).unwrap()
    );
    assert_eq!(
        build[0]["platforms"],
        toml::Value::try_from(["linux", "macos"]).unwrap()
    );
    assert_eq!(
        build[1]["command"],
        toml::Value::try_from([
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "install.ps1"
        ])
        .unwrap()
    );
    assert_eq!(
        build[1]["platforms"],
        toml::Value::try_from(["windows"]).unwrap()
    );
    let mut commands = 0;
    for table in ["actions", "panes", "startup", "events", "link_handlers"] {
        for entry in manifest
            .get(table)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            let command = entry["command"].as_array().unwrap();
            assert_eq!(command[0].as_str(), Some("./bin/herdr-ank"), "[[{table}]]");
            commands += 1;
        }
    }
    assert!(commands > 0);
}

#[test]
fn bin_is_ignored_by_git() {
    let out = Command::new("git")
        .args(["check-ignore", "-q", "bin/herdr-ank"])
        .current_dir(ROOT)
        .status()
        .unwrap();
    assert!(out.success());
}
