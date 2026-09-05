//! A path the guard cannot list, and a file it cannot read, become reported findings.

use std::fs;

use crate::scan::{offending_lines, rust_files};

#[test]
fn a_directory_that_cannot_be_listed_becomes_a_finding() {
    let made = tempfile::TempDir::new();
    assert!(
        made.is_ok(),
        "the fixture needs a temporary directory to live in"
    );
    let Ok(temp) = made else { return };
    let file = temp.path().join("plain_file.rs");
    let written = fs::write(&file, "// a source file with nothing forbidden in it\n");
    assert!(
        written.is_ok(),
        "the fixture file {} must be writable",
        file.display()
    );
    let shown = file.display().to_string();

    let mut findings: Vec<String> = Vec::new();
    let walked = rust_files(&file, &mut findings);
    assert!(
        walked.is_empty(),
        "a path that is not a directory holds no source files, so the walk is empty: {walked:?}"
    );
    assert_eq!(
        findings.len(),
        1,
        "one path that cannot be listed is one finding, not none and not many: {findings:?}"
    );
    let finding = findings.first().map_or("", String::as_str);
    assert!(
        finding.contains(&shown),
        "the finding names the path it could not list: {finding}"
    );
    assert!(
        finding.contains("cannot be listed"),
        "the finding carries the listing failure: {finding}"
    );

    let mut clean: Vec<String> = Vec::new();
    let found = rust_files(temp.path(), &mut clean);
    assert!(
        clean.is_empty(),
        "a directory that lists produces no finding: {clean:?}"
    );
    assert_eq!(
        found,
        vec![file],
        "the walk over a listable directory finds the one source file in it"
    );
}

#[test]
fn a_source_file_that_is_not_utf8_becomes_a_finding() {
    let made = tempfile::TempDir::new();
    assert!(
        made.is_ok(),
        "the fixture needs a temporary directory to live in"
    );
    let Ok(temp) = made else { return };
    let broken = temp.path().join("not_utf8.rs");
    let written = fs::write(&broken, [0xff, 0xfe, 0xfd]);
    assert!(
        written.is_ok(),
        "the fixture file {} must be writable",
        broken.display()
    );
    let shown = broken.display().to_string();

    let outcome = offending_lines(&broken);
    assert!(
        outcome.is_err(),
        "bytes that are not UTF-8 cannot be read as text, so {shown} is a finding"
    );
    let finding = outcome.err().unwrap_or_default();
    assert!(
        finding.contains(&shown),
        "the finding names the file it could not read: {finding}"
    );
    assert!(
        finding.contains("cannot be read"),
        "the finding carries the read failure: {finding}"
    );

    let readable = temp.path().join("readable.rs");
    let saved = fs::write(&readable, "// a source file with nothing forbidden in it\n");
    assert!(
        saved.is_ok(),
        "the fixture file {} must be writable",
        readable.display()
    );
    let lines = offending_lines(&readable);
    assert!(
        lines.is_ok(),
        "a file that is UTF-8 is read rather than reported, so a finding means the fixture \
         and not every call"
    );
    assert!(
        lines.unwrap_or_default().is_empty(),
        "a readable file naming nothing forbidden has no offending line"
    );
}
