//! CI workflows stay on the static green subset: no `dynamic_linking`, no `dev_tools`.

use std::{collections::BTreeSet, fs};

use crate::{
    run_steps::run_commands,
    tree::{repo_root, workflow_files},
};

const STATIC_ONLY_BAN: &str = "dynamic_linking";

const REQUIRED_COMMANDS: [(&str, &str); 2] = [
    (".github/workflows/test.yml", "cargo test --workspace"),
    (
        ".github/workflows/clippy.yml",
        "cargo clippy --workspace --all-targets -- -D warnings",
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
fn every_ci_workspace_command_is_static() {
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
            if command.contains(STATIC_ONLY_BAN) {
                violations.insert(format!(
                    "STATIC {file} — `{command}` names `{STATIC_ONLY_BAN}`; CI green is the \
                     static subset"
                ));
            }
            if command.contains("net_qa") || command.contains("schema") {
                violations.insert(format!(
                    "STALE {file} — `{command}` still names net_qa/schema features"
                ));
            }
            reached.insert((
                file.clone(),
                command.split_whitespace().collect::<Vec<_>>().join(" "),
            ));
        }
    }
    for (file, command) in REQUIRED_COMMANDS {
        let pair = (
            file.to_owned(),
            command.split_whitespace().collect::<Vec<_>>().join(" "),
        );
        if !reached.contains(&pair) {
            violations.insert(format!("UNREACHED {file} — never saw `{command}`"));
        }
    }
    assert!(
        violations.is_empty(),
        "CI workspace feature guard failed:\n{}",
        violations.into_iter().collect::<Vec<_>>().join("\n")
    );
}
