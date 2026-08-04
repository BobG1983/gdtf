use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    refs::{link_targets, root_anchored},
    tree::{repo_root, scanned_docs},
};

const SKIP_PAIRS: [(&str, &str); 6] = [
    (
        "docs/decisions/0001-rust-bevy-rewrite.md",
        "docs/architecture.md",
    ),
    (".claude/rules/git-workflow.md", ".claude/.gate-pass"),
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

fn anchored_resolves(root: &Path, reference: &str) -> bool {
    if root.join(reference).exists() {
        return true;
    }
    reference.starts_with("content/") && root.join("assets").join(reference).exists()
}

fn link_resolves(root: &Path, file: &str, target: &str) -> bool {
    if let Some(rooted) = target.strip_prefix('/') {
        return root.join(rooted).exists();
    }
    let dir = Path::new(file).parent().unwrap_or_else(|| Path::new(""));
    root.join(dir).join(target).exists()
}

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
            continue; 
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
        "dead path references (docs path-truth gate):\n{rendered}"
    );
}
