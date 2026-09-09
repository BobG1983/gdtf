//! Fail if any file under `libs/`, tests included, carries the game's vocabulary.

use std::{fs, path::Path};

use crate::libs_layer::tree::{LIBS_DIR, repo_root, tracked_files};

// Words and wire literals that belong to the game these crates were extracted from, matched
// case-insensitively.
const GAME_WORDS: [&str; 12] = [
    "gdtf",
    "grimdark",
    "turfwar",
    "ganger",
    "necromunda",
    "hivescape",
    "battlescape",
    "aftermath",
    "app.phase",
    "app.capture",
    "capture.screenshot",
    "appphase",
];

// The one file that holds these strings in order to forbid them.
const ALLOWED: [&str; 1] = ["libs/cobalt_mcp_server/tests/mcp_server_suite/game_free/scan.rs"];

const RULE: &str = ".claude/rules/libs-layer.md";

// Every line of `path` naming a game word. A file that is not UTF-8 holds no such word.
fn offending_lines(root: &Path, path: &str) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let lowered = line.to_lowercase();
        if let Some(word) = GAME_WORDS.iter().find(|word| lowered.contains(**word)) {
            found.push(format!(
                "{path}:{} — `{word}` in {}",
                number + 1,
                line.trim()
            ));
        }
    }
    found
}

#[test]
fn no_file_under_libs_names_the_game() {
    let root = repo_root();
    let files = tracked_files(&root, LIBS_DIR);
    assert!(
        !files.is_empty(),
        "no file found under {}/ in {} — enumeration is broken, and a guard that reads \
         nothing is not a pass",
        LIBS_DIR,
        root.display()
    );

    let violations: Vec<String> = files
        .iter()
        .filter(|path| !ALLOWED.contains(&path.as_str()))
        .flat_map(|path| offending_lines(&root, path))
        .collect();

    assert!(
        violations.is_empty(),
        "crates under {LIBS_DIR}/ carry nothing game-specific, in tests as much as in src, so \
         no file there may name {GAME_WORDS:?}:\n{}\n\nInvent a fixture that models nothing in \
         the game, or move the thing being named into a crate under crates/. The rule is \
         {RULE}.",
        violations.join("\n")
    );
}
