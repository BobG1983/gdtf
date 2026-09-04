//! No source file in this crate carries a game name.

use std::path::{Path, PathBuf};

// Words that belong to the game this bridge was extracted from, matched case-insensitively.
const GAME_WORDS: [&str; 2] = ["gdtf", "grimdark"];

const RULE: &str = ".claude/rules/libs-layer.md";

fn source_root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).to_path_buf()
}

// Every `.rs` file under `dir`, walked without recursion into anything but directories.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        unreachable!(
            "the crate's own source directory is readable at {}",
            dir.display()
        );
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            found.push(path);
        }
    }
    found
}

fn offending_lines(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!(
            "a source file this crate compiles is readable at {}",
            path.display()
        );
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
    found
}

#[test]
fn no_source_file_in_this_crate_names_the_game() {
    let files = rust_files(&source_root());
    assert!(
        !files.is_empty(),
        "no source file was found under {}, so this guard is reading nothing",
        source_root().display()
    );

    let violations: Vec<String> = files
        .iter()
        .flat_map(|path| offending_lines(path))
        .collect();

    assert!(
        violations.is_empty(),
        "this crate is game-free, so no source file may name {GAME_WORDS:?}. The caller \
         supplies the server name, the host names and the channel variables:\n{}\n\nThe rule \
         is {RULE}.",
        violations.join("\n")
    );
}
