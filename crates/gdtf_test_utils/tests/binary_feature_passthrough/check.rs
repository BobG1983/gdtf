//! The passthrough test — walks every tracked binary manifest, derives the
//! declaration each `net_qa`-bearing library dependency obliges it to make, and
//! carries the whole violation list in one final assert (see the suite doc in
//! `main.rs`).

use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    manifest::{feature_declaration, path_dependencies},
    tree::{binary_manifests, repo_root},
};

/// The feature this guard is about: the DEV-ONLY QA network control channel.
const QA_FEATURE: &str = "net_qa";

/// The `(binary manifest, expected declaration)` pairs the derived walk MUST
/// have reached and found satisfied.
///
/// The walk above is the general rule; this is its liveness check. Without it a
/// manifest reader that quietly stopped recognising a dependency line would
/// turn the guard green by checking nothing at all — the exact failure mode
/// GTW-878 was filed about, one layer up.
const REQUIRED_PAIRS: [(&str, &str); 2] = [
    (
        "bins/gdtf_content_editor/Cargo.toml",
        "net_qa = [\"gdtf_content_editor/net_qa\"]",
    ),
    (
        "bins/grimdark_turfwar/Cargo.toml",
        "net_qa = [\"gdtf_app/net_qa\"]",
    ),
];

/// The GTW-878 binary-feature-passthrough guard (see the suite doc in
/// `main.rs`).
#[test]
fn every_binary_passes_through_its_library_net_qa_feature() {
    let root = repo_root();
    let manifests = binary_manifests(&root);
    let mut violations: BTreeSet<String> = BTreeSet::new();
    let mut satisfied: BTreeSet<(String, String)> = BTreeSet::new();
    assert!(
        !manifests.is_empty(),
        "no tracked bins/*/Cargo.toml found under {} — enumeration is broken",
        root.display()
    );
    for bin_manifest in &manifests {
        let Ok(bin_text) = fs::read_to_string(root.join(bin_manifest)) else {
            continue; // tracked but deleted from the working tree — nothing to read
        };
        let bin_dir = Path::new(bin_manifest)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        for (dep_name, dep_path) in path_dependencies(&bin_text) {
            let lib_manifest = root.join(bin_dir).join(&dep_path).join("Cargo.toml");
            let Ok(lib_text) = fs::read_to_string(&lib_manifest) else {
                continue; // a path dependency outside this checkout — not ours to judge
            };
            if feature_declaration(&lib_text, QA_FEATURE).is_none() {
                continue; // the library offers no QA channel — nothing to pass through
            }
            let expected = format!("{QA_FEATURE} = [\"{dep_name}/{QA_FEATURE}\"]");
            match feature_declaration(&bin_text, QA_FEATURE) {
                Some(actual) if actual == expected => {
                    satisfied.insert((bin_manifest.clone(), expected));
                }
                Some(actual) => {
                    violations.insert(format!(
                        "WRONG {bin_manifest} declares `{actual}` — expected `{expected}`"
                    ));
                }
                None => {
                    violations.insert(format!(
                        "MISSING {bin_manifest} — `{dep_name}` declares a `{QA_FEATURE}` feature, \
                         so the binary must declare `{expected}` or the channel is unreachable \
                         from any launchable binary"
                    ));
                }
            }
        }
    }
    for (bin_manifest, expected) in REQUIRED_PAIRS {
        let pair = (bin_manifest.to_owned(), expected.to_owned());
        if !satisfied.contains(&pair) {
            violations.insert(format!(
                "UNREACHED {bin_manifest} — the walk never confirmed `{expected}`; either the \
                 declaration is gone or the manifest reader stopped seeing it"
            ));
        }
    }
    for line in &violations {
        eprintln!("{line}");
    }
    let rendered = violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        violations.is_empty(),
        "binary feature passthrough violations (GTW-878):\n{rendered}"
    );
}
