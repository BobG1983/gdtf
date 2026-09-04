//! Fail if the QA host is gated on `debug_assertions` instead of the `mcp` feature.

use crate::tree::{GATE_FILES, QA_PATHS, read, repo_root, tracked_qa_sources};

const PROFILE_GATE: &str = "debug_assertions";

const FEATURE_GATE: &str = "cfg(feature = \"mcp\")";

const RULE: &str = ".claude/rules/verification.md";

#[test]
fn no_qa_source_is_gated_on_the_build_profile() {
    let root = repo_root();
    let sources = tracked_qa_sources(&root);
    assert!(
        !sources.is_empty(),
        "no `.rs` file found under {QA_PATHS:?} in {} — enumeration is broken, and a guard \
         that reads nothing is not a pass",
        root.display()
    );
    let offenders: Vec<&String> = sources
        .iter()
        .filter(|path| read(&root, path).contains(PROFILE_GATE))
        .collect();
    assert!(
        offenders.is_empty(),
        "the QA host is compiled in by each host package's `mcp` feature, so nothing under \
         {QA_PATHS:?} may name `{PROFILE_GATE}`:\n{offenders:#?}\n\nA `{PROFILE_GATE}` gate \
         drops the host from every release build, which is the regime the `mcp` feature \
         replaced. The rule is {RULE}."
    );
}

#[test]
fn every_file_that_declares_the_qa_host_gates_it_on_the_mcp_feature() {
    let root = repo_root();
    let missing: Vec<&str> = GATE_FILES
        .into_iter()
        .filter(|path| !read(&root, path).contains(FEATURE_GATE))
        .collect();
    assert!(
        missing.is_empty(),
        "each file that declares or wires the QA host carries `#[{FEATURE_GATE}]`, and these \
         do not:\n{missing:#?}\n\nThe rule is {RULE}."
    );
}
