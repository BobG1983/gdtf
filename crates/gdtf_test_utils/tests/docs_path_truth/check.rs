//! The path-truth test — resolves every extracted reference against the live
//! tree and carries the dead list in one final assert (see the suite doc in
//! `main.rs`).

use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    refs::{link_targets, root_anchored},
    tree::{repo_root, scanned_docs},
};

/// Pinned `(file, reference)` pairs that are DELIBERATELY unresolvable — each
/// entry names a path whose absence is the documented point, not stranded
/// canon. Grow this list only with a stated reason.
const SKIP_PAIRS: [(&str, &str); 6] = [
    // ADR-0001 RECORDS the deliberate deletion of docs/architecture.md
    // (commit 5d5dcef) — the deleted path is the subject of the record.
    (
        "docs/decisions/0001-rust-bevy-rewrite.md",
        "docs/architecture.md",
    ),
    // The gate-pass marker is a transient artifact written by /gate and
    // consumed by /land — it legitimately does not exist between gate runs.
    (".claude/rules/git-workflow.md", ".claude/.gate-pass"),
    // ADR-0007 is `Superseded by 0008`, and GTW-943 deleted the very surface it
    // describes — the per-family editor query pair, the fifteen-variant request
    // enum, and the battle read-model. Its Context and Decision sections quote
    // those files by path, and this directory's rule 2 keeps a decided ADR's
    // body exactly as written, so the paths stay and the absence is the point.
    (
        "docs/decisions/0007-net-qa-command-discoverability.md",
        "crates/gdtf_content_editor/src/net_qa/router.rs",
    ),
    (
        "docs/decisions/0007-net-qa-command-discoverability.md",
        "crates/gdtf_content_editor/src/net_qa/snapshot/topics.rs",
    ),
    (
        "docs/decisions/0007-net-qa-command-discoverability.md",
        "crates/gdtf_qa_protocol/src/envelope/request.rs",
    ),
    (
        "docs/decisions/0007-net-qa-command-discoverability.md",
        "crates/gdtf_qa_protocol/src/view/battle.rs",
    ),
];

/// Whether a root-anchored `reference` resolves — from the repo root, or (for
/// `content/…` runs, which docs quote asset-relative) under `assets/`.
fn anchored_resolves(root: &Path, reference: &str) -> bool {
    if root.join(reference).exists() {
        return true;
    }
    reference.starts_with("content/") && root.join("assets").join(reference).exists()
}

/// Whether a markdown link `target` resolves relative to `file` (or to the
/// repo root when the target starts with `/`).
fn link_resolves(root: &Path, file: &str, target: &str) -> bool {
    if let Some(rooted) = target.strip_prefix('/') {
        return root.join(rooted).exists();
    }
    let dir = Path::new(file).parent().unwrap_or_else(|| Path::new(""));
    root.join(dir).join(target).exists()
}

/// The GTW-626 docs path-truth gate (see the suite doc in `main.rs`).
#[test]
fn every_referenced_path_resolves() {
    let root = repo_root();
    let files = scanned_docs(&root);
    let mut dead: BTreeSet<String> = BTreeSet::new();
    assert!(
        !files.is_empty(),
        "no tracked docs found under {} — enumeration is broken",
        root.display()
    );
    for file in &files {
        let Ok(bytes) = fs::read(root.join(file)) else {
            continue; // tracked but deleted from the working tree — nothing to scan
        };
        let text = String::from_utf8_lossy(&bytes);
        for reference in root_anchored(&text) {
            if SKIP_PAIRS.contains(&(file.as_str(), reference.as_str())) {
                continue;
            }
            if !anchored_resolves(&root, &reference) {
                dead.insert(format!("DEAD {file} -> {reference}"));
            }
        }
        for target in link_targets(&text) {
            if !link_resolves(&root, file, &target) {
                dead.insert(format!("DEADLINK {file} -> {target}"));
            }
        }
    }
    for line in &dead {
        eprintln!("{line}");
    }
    let rendered = dead.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dead.is_empty(),
        "dead path references (docs path-truth gate, GTW-626):\n{rendered}"
    );
}
