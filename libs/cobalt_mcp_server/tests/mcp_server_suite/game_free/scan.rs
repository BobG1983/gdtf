//! No source file in this crate carries a game name.

use std::path::{Path, PathBuf};

// Words that belong to the game this bridge was extracted from, matched case-insensitively.
const GAME_WORDS: [&str; 2] = ["gdtf", "grimdark"];

const RULE: &str = ".claude/rules/libs-layer.md";

fn source_root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).to_path_buf()
}

// Every `.rs` file under `dir`, walked without recursion into anything but directories. A
// directory that cannot be listed becomes a finding, and the walk carries on with the rest.
pub(crate) fn rust_files(dir: &Path, findings: &mut Vec<String>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            findings.push(format!("{} — cannot be listed: {error}", dir.display()));
            return found;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_files(&path, findings));
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            found.push(path);
        }
    }
    found
}

// Every line of `path` naming a game word, or a finding saying why the file could not be read.
pub(crate) fn offending_lines(path: &Path) -> Result<Vec<String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Err(format!("{} — cannot be read: {error}", path.display())),
    };
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let lowered = line.to_lowercase();
        for word in GAME_WORDS {
            if lowered.contains(word) {
                found.push(format!(
                    "{}:{} — {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    Ok(found)
}

#[test]
fn no_source_file_in_this_crate_names_the_game() {
    let mut violations: Vec<String> = Vec::new();
    let files = rust_files(&source_root(), &mut violations);
    assert!(
        !files.is_empty(),
        "no source file was found under {}, so this guard is reading nothing. Findings so \
         far:\n{}",
        source_root().display(),
        violations.join("\n")
    );

    for path in &files {
        match offending_lines(path) {
            Ok(found) => violations.extend(found),
            Err(finding) => violations.push(finding),
        }
    }

    assert!(
        violations.is_empty(),
        "this crate is game-free, so no source file may name {GAME_WORDS:?}. The caller \
         supplies the server name, the host names and the channel variables. A file this guard \
         could not list or read is a failure too, because a file it skipped is one it did not \
         check:\n{}\n\nThe rule is {RULE}.",
        violations.join("\n")
    );
}
