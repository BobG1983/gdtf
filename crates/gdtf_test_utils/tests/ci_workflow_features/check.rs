//! The CI-workflow feature test — walks every tracked workflow file, collects
//! the workspace-wide `cargo` commands, and carries the whole violation list in
//! one final assert (see the suite doc in `main.rs`).

use std::{collections::BTreeSet, fs};

use crate::{
    run_steps::run_commands,
    tree::{repo_root, workflow_files},
};

/// The GAME's QA control channel feature, as a workflow command must name it.
const GAME_FEATURE: &str = "grimdark_turfwar/net_qa";

/// The EDITOR's QA control channel feature, as a workflow command must name it.
///
/// `gdtf_content_editor` here is the LIBRARY package
/// (`crates/gdtf_content_editor`), not the binary package
/// (`gdtf_content_editor_bin`) — a package-qualified feature turns on only that
/// package's, and the library is where `src/net_qa/` and the two gated test
/// binaries live.
const EDITOR_FEATURE: &str = "gdtf_content_editor/net_qa";

/// Dev-only fast-linking that must never reach a CI command — CI green is the
/// static subset per `CLAUDE.md`.
const STATIC_ONLY_BAN: &str = "dynamic_linking";

/// The workspace-wide CI commands the derived walk MUST have reached.
///
/// The walk is the general rule; this is its liveness check. Without it a line
/// reader that stopped recognising `run:` commands would turn the guard green
/// by checking nothing at all.
const REQUIRED_COMMANDS: [(&str, &str); 2] = [
    (
        ".github/workflows/test.yml",
        "cargo test --workspace --features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa",
    ),
    (
        ".github/workflows/clippy.yml",
        "cargo clippy --workspace --all-targets --features \
         grimdark_turfwar/net_qa,gdtf_content_editor/net_qa -- -D warnings",
    ),
];

/// Every `cargo … --workspace …` command a workflow step RUNS, in both the
/// single-line and block `run:` forms (see `run_steps.rs`), cut at the `cargo `
/// word so a shell prefix does not read as part of the command.
fn workspace_cargo_commands(text: &str) -> Vec<String> {
    run_commands(text)
        .into_iter()
        .filter(|command| command.contains("cargo ") && command.contains("--workspace"))
        .filter_map(|command| {
            command
                .split_once("cargo ")
                .map(|(_, rest)| format!("cargo {rest}"))
        })
        .collect()
}

/// The GTW-883 CI-workflow feature guard (see the suite doc in `main.rs`).
#[test]
fn every_ci_workspace_command_names_both_net_qa_features() {
    let root = repo_root();
    let files = workflow_files(&root);
    let mut violations: BTreeSet<String> = BTreeSet::new();
    let mut reached: BTreeSet<(String, String)> = BTreeSet::new();
    assert!(
        !files.is_empty(),
        "no tracked .github/workflows/*.yml found under {} — enumeration is broken",
        root.display()
    );
    for file in &files {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue; // tracked but deleted from the working tree — nothing to read
        };
        for command in workspace_cargo_commands(&text) {
            for feature in [GAME_FEATURE, EDITOR_FEATURE] {
                if !command.contains(feature) {
                    violations.insert(format!(
                        "MISSING {file} — `{command}` does not name `{feature}`; without it that \
                         package's `net_qa` module compiles nowhere in CI and its gated test \
                         binaries report 0 tests"
                    ));
                }
            }
            if command.contains(STATIC_ONLY_BAN) {
                violations.insert(format!(
                    "STATIC {file} — `{command}` names `{STATIC_ONLY_BAN}`; CI green is the \
                     static subset"
                ));
            }
            reached.insert((file.clone(), command));
        }
    }
    for (file, command) in REQUIRED_COMMANDS {
        let pair = (
            file.to_owned(),
            command.split_whitespace().collect::<Vec<_>>().join(" "),
        );
        if !reached.contains(&pair) {
            violations.insert(format!(
                "UNREACHED {file} — the walk never saw `{}`; either the command changed or the \
                 line reader stopped seeing it",
                pair.1
            ));
        }
    }
    for line in &violations {
        eprintln!("{line}");
    }
    let rendered = violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        violations.is_empty(),
        "CI workflow feature violations (GTW-883):\n{rendered}"
    );
}
