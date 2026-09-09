//! A manifest the guard cannot read or parse becomes a reported finding, not an abort.

use std::fs;

use crate::libs_layer::{check::read_manifest, tree::repo_root};

#[test]
fn a_manifest_that_is_not_valid_toml_becomes_a_finding() {
    let made = tempfile::TempDir::new();
    assert!(
        made.is_ok(),
        "the fixture needs a temporary directory to live in"
    );
    let Ok(temp) = made else { return };
    let path = "libs/broken/Cargo.toml";
    let dir = temp.path().join("libs/broken");
    let built = fs::create_dir_all(&dir);
    assert!(
        built.is_ok(),
        "the fixture directory {} must be creatable",
        dir.display()
    );
    let written = fs::write(dir.join("Cargo.toml"), "this line is not toml\n");
    assert!(
        written.is_ok(),
        "the fixture manifest under {} must be writable",
        dir.display()
    );

    let outcome = read_manifest(temp.path(), path);
    assert!(
        outcome.is_err(),
        "text that is not TOML cannot parse as a manifest, so {path} is a finding"
    );
    let finding = outcome.err().unwrap_or_default();
    assert!(
        finding.contains(path),
        "the finding names the manifest it could not parse: {finding}"
    );
    assert!(
        finding.contains("is not valid TOML"),
        "the finding carries the parse failure: {finding}"
    );

    let readable = read_manifest(&repo_root(), "Cargo.toml");
    assert!(
        readable.is_ok(),
        "the workspace manifest reads and parses, so a finding means the fixture and not \
         every call"
    );
}

#[test]
fn a_manifest_that_cannot_be_read_becomes_a_finding() {
    let made = tempfile::TempDir::new();
    assert!(
        made.is_ok(),
        "the fixture needs a temporary directory to live in"
    );
    let Ok(temp) = made else { return };
    let path = "libs/absent/Cargo.toml";

    let outcome = read_manifest(temp.path(), path);
    assert!(
        outcome.is_err(),
        "nothing was written at {path}, so the read fails and the guard reports it"
    );
    let finding = outcome.err().unwrap_or_default();
    assert!(
        finding.contains(path),
        "the finding names the manifest it could not read: {finding}"
    );
    assert!(
        finding.contains("cannot be read"),
        "the finding carries the read failure: {finding}"
    );

    let readable = read_manifest(&repo_root(), "Cargo.toml");
    assert!(
        readable.is_ok(),
        "the workspace manifest reads and parses, so a finding means the fixture and not \
         every call"
    );
}
