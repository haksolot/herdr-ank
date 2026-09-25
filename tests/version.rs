use std::fs;
use std::path::Path;

fn version_in(file: &str, table: Option<&str>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = fs::read_to_string(&path).unwrap();
    let doc: toml::Table = text.parse().unwrap();
    let scope = match table {
        Some(name) => doc[name].as_table().unwrap(),
        None => &doc,
    };
    scope["version"].as_str().unwrap().to_string()
}

#[test]
fn cargo_and_manifest_carry_the_same_version() {
    let cargo = version_in("Cargo.toml", Some("package"));
    let manifest = version_in("herdr-plugin.toml", None);
    assert_eq!(
        cargo, manifest,
        "Cargo.toml says {cargo} but herdr-plugin.toml says {manifest}: \
         the release workflow refuses a tag until both agree"
    );
}

#[test]
fn release_workflow_builds_the_archives_install_sh_expects() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    let workflow =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));
    for target in [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ] {
        assert!(
            workflow.contains(target),
            "release.yml never builds {target}"
        );
    }
    assert!(
        workflow.contains("herdr-ank-${VERSION}-${TARGET}.tar.gz"),
        "release.yml does not name its archives herdr-ank-<version>-<target>.tar.gz"
    );
    assert!(
        workflow.contains("SHA256SUMS"),
        "release.yml publishes no SHA256SUMS"
    );
}
