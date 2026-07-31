//! The GTW-939 guard: CI must run the `gdtf_qa_protocol` `schema` feature, package-scoped.
//!
//! The sibling `check.rs` guards the WORKSPACE commands — that every one names both
//! `net_qa` features. This file guards a different thing: that the two PACKAGE-SCOPED
//! `schema` steps exist at all. A workspace walk cannot see them, because they must not
//! carry `--workspace`: cargo builds one `gdtf_qa_protocol` library unit per invocation, so
//! naming `gdtf_qa_protocol/schema` on a workspace command would unify schemars into
//! `bins/gdtf_qa_mcp`, which carries schema documents as opaque text and must never link a
//! schema library.
//!
//! Without this guard, deleting either step leaves the whole suite green while the
//! `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` derives across
//! `crates/gdtf_qa_protocol/src/ids/` and the whole `src/ids/test/schema.rs` module go back
//! to compiling, linting and running NOWHERE in CI — the GTW-790 / GTW-877 defect class in
//! a third crate.

use std::{collections::BTreeSet, fs};

use crate::{
    run_steps::run_commands,
    tree::{repo_root, workflow_files},
};

/// The package the `schema` feature belongs to — the scope every step must name.
const SCHEMA_PACKAGE: &str = "-p gdtf_qa_protocol";

/// The feature flag as a package-scoped command names it (unqualified: `-p` already fixed
/// which package's feature it is).
const SCHEMA_FEATURE: &str = "--features schema";

/// The package-scoped `schema` commands CI must run, whitespace-normalised.
///
/// Both are here rather than the test step alone: the workspace lints are denied for
/// feature-on code exactly as they are for anything else, and the workspace clippy step
/// never compiles it.
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

/// Every command a workflow step RUNS, cut at the `cargo ` word so a shell prefix does not
/// read as part of the command. Unlike `check.rs`'s reader this does NOT filter on
/// `--workspace` — the commands it looks for are deliberately package-scoped.
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

/// CI runs the `schema` feature, and runs it package-scoped (GTW-939).
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
            continue; // tracked but deleted from the working tree — nothing to read
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
