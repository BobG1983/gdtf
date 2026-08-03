//! `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` derives across
use std::{collections::BTreeSet, fs};

use crate::{
    run_steps::run_commands,
    tree::{repo_root, workflow_files},
};

const SCHEMA_PACKAGE: &str = "-p gdtf_qa_protocol";

const SCHEMA_FEATURE: &str = "--features schema";

const REQUIRED_SCHEMA_COMMANDS: [(&str, &str); 2] = [
    (
        ".github/workflows/test.yml",
        "cargo test -p gdtf_qa_protocol --features schema",
    ),
    (
        ".github/workflows/clippy.yml",
        "cargo clippy -p gdtf_qa_protocol --all-targets --features schema -- -D warnings",
    ),
];

fn cargo_commands(text: &str) -> Vec<String> {
    run_commands(text)
        .into_iter()
        .filter(|command| command.contains("cargo "))
        .filter_map(|command| {
            command
                .split_once("cargo ")
                .map(|(_, rest)| format!("cargo {rest}"))
        })
        .collect()
}

#[test]
fn ci_runs_the_schema_feature_package_scoped() {
    let root = repo_root();
    let files = workflow_files(&root);
    assert!(
        !files.is_empty(),
        "no tracked .github/workflows/*.yml found under {} — enumeration is broken",
        root.display()
    );
    let mut violations: BTreeSet<String> = BTreeSet::new();
    let mut reached: BTreeSet<(String, String)> = BTreeSet::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue; 
        };
        for command in cargo_commands(&text) {
            if command.contains(SCHEMA_FEATURE) && !command.contains(SCHEMA_PACKAGE) {
                violations.insert(format!(
                    "SCOPE {file} — `{command}` enables the schema feature without \
                     `{SCHEMA_PACKAGE}`; cargo builds one gdtf_qa_protocol library unit per \
                     invocation, so a wider scope links schemars into bins/gdtf_qa_mcp"
                ));
            }
            reached.insert((file.clone(), command));
        }
    }
    for (file, command) in REQUIRED_SCHEMA_COMMANDS {
        let pair = (file.to_owned(), command.to_owned());
        if !reached.contains(&pair) {
            violations.insert(format!(
                "MISSING {file} — the walk never saw `{command}`; without that step the \
                 `schema`-gated derives in gdtf_qa_protocol's src/ids/ and the whole \
                 src/ids/test/schema.rs module compile, lint and run nowhere in CI"
            ));
        }
    }
    for line in &violations {
        eprintln!("{line}");
    }
    let rendered = violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        violations.is_empty(),
        "CI schema-feature violations (GTW-939):\n{rendered}"
    );
}
