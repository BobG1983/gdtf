use std::{collections::BTreeSet, fs};

use crate::{
    run_steps::run_commands,
    tree::{repo_root, workflow_files},
};

const GAME_FEATURE: &str = "grimdark_turfwar/net_qa";

const EDITOR_FEATURE: &str = "gdtf_content_editor/net_qa";

const STATIC_ONLY_BAN: &str = "dynamic_linking";

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
            continue; 
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
        "CI workflow feature violations:\n{rendered}"
    );
}
