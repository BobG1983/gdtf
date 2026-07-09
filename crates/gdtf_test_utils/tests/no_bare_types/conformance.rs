//! The GTW-599 tree-wide conformance guard: walk every tracked production
//! `.rs` file, suppress registered exemptions, and fail on any residual
//! violation or stale registry entry. The hermetic checker-behavior tests
//! (AC2) live in [`crate::fixtures`].

use std::{collections::BTreeSet, fs};

use crate::{
    registry::Registry,
    tree::{production_rs, workspace_root},
    types::Violation,
    walk::scan_source,
};

/// Walk every tracked production `.rs` file, flag bare domain types per
/// `.claude/rules/no-bare-types.md` (rules 4/5 + the Q4 carve-out encoded in
/// `walk`), suppress registered exemptions, and fail on any residual violation
/// or stale registry entry. Deterministic output — one diagnostic per block,
/// ordered by `(path, line, column)`.
#[test]
fn no_bare_types_conformance() {
    let root = workspace_root();
    let files = production_rs(&root);
    assert!(
        !files.is_empty(),
        "no tracked production .rs files found under {}",
        root.display()
    );

    let registry = Registry::load(&root);
    let mut violations = Vec::new();
    let mut parse_errors = Vec::new();
    for path in &files {
        let Ok(src) = fs::read_to_string(root.join(&**path)) else {
            continue; // tracked but absent from the working tree
        };
        match scan_source(path, &src) {
            Ok(found) => violations.extend(found),
            Err(err) => parse_errors.push(format!("{}: {}", *err.path, err.message)),
        }
    }
    violations.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
            .then_with(|| a.type_name.cmp(&b.type_name))
    });

    let live_keys: BTreeSet<String> = violations.iter().map(Violation::key).collect();
    let stale = registry.stale(&live_keys);

    for err in &parse_errors {
        eprintln!("no-bare-types: WARN could not parse {err}");
    }

    let unsuppressed: Vec<&Violation> = violations
        .iter()
        .filter(|v| !registry.contains(&v.key()))
        .collect();
    for violation in &unsuppressed {
        eprintln!("{}", violation.render());
    }
    for entry in &stale {
        eprintln!("no-bare-types: STALE exemption — remove `{entry}`");
    }

    let rendered = unsuppressed
        .iter()
        .map(|v| v.render())
        .collect::<Vec<_>>()
        .join("\n");
    let stale_note = if stale.is_empty() {
        String::new()
    } else {
        format!("\nstale exemptions to remove:\n{}", stale.join("\n"))
    };
    assert!(
        unsuppressed.is_empty() && stale.is_empty(),
        "no-bare-types violations (.claude/rules/no-bare-types.md):\n{rendered}{stale_note}"
    );
}
