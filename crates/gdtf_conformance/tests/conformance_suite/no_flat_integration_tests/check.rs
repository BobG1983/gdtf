//! Fail if any crate gains a flat integration-test binary, a second suite binary, or a suite
//! binary not named `<name>_suite`.

use std::collections::BTreeMap;

use crate::no_flat_integration_tests::tree::{flat_test_files, repo_root, suite_roots};

// Flat test files allowed to stay. Each one was folded into its crate's suite, so none remain.
const ALLOWED_FLATS: &[&str] = &[];

#[test]
fn no_unexpected_flat_integration_tests() {
    let root = repo_root();
    assert!(
        root.join("crates").is_dir(),
        "no crates directory under {} — this guard is reading the wrong directory and \
         cannot see any test files",
        root.display()
    );
    let found: Vec<String> = flat_test_files(&root)
        .into_iter()
        .filter(|path| !ALLOWED_FLATS.contains(&path.as_str()))
        .collect();
    assert!(
        found.is_empty(),
        "these test files sit loose in a tests/ directory:\n{}\n\nEach one builds its own test \
         binary and links Bevy again, which makes the suite slow. Move it into the crate's one \
         suite, tests/<crate>_suite/main.rs, with the test in a module beside it. If it really \
         has to run on its own, add it to ALLOWED_FLATS in \
         crates/gdtf_conformance/tests/conformance_suite/no_flat_integration_tests/check.rs. \
         The policy is in docs/testing.md and .claude/rules/module-layout.md rule 5.",
        found.join("\n")
    );
}

#[test]
fn every_crate_has_at_most_one_suite_binary_named_for_it() {
    let root = repo_root();
    let roots = suite_roots(&root);
    assert!(
        roots
            .iter()
            .any(|p| p.starts_with("crates/gdtf_conformance/tests/")),
        "the suite-root scan under {} did not find this guard's own binary, so it is reading \
         the wrong directory",
        root.display()
    );
    let mut per_crate: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut misnamed = Vec::new();
    for path in roots {
        let parts: Vec<&str> = path.split('/').collect();
        let (Some(crate_dir), Some(dir)) = (parts.get(1), parts.get(3)) else {
            continue;
        };
        if !dir.ends_with("_suite") {
            misnamed.push(path.clone());
        }
        per_crate
            .entry((*crate_dir).to_owned())
            .or_default()
            .push(path);
    }
    let extra: Vec<String> = per_crate
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(crate_dir, paths)| format!("{crate_dir}: {}", paths.join(", ")))
        .collect();
    assert!(
        misnamed.is_empty() && extra.is_empty(),
        "integration-test binaries not named <name>_suite:\n{}\n\ncrates with more than one \
         integration-test binary:\n{}\n\nEach tests/<dir>/main.rs is a binary that compiles and \
         links Bevy again. A crate keeps one, tests/<name>_suite/main.rs, and declares each \
         suite as a module inside it. The policy is in docs/testing.md and \
         .claude/rules/module-layout.md rule 5.",
        misnamed.join("\n"),
        extra.join("\n")
    );
}
