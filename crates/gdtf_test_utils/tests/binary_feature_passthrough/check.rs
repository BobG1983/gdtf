use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    manifest::{feature_declaration, path_dependencies},
    tree::{binary_manifests, repo_root},
};

const QA_FEATURE: &str = "net_qa";

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
            continue; 
        };
        let bin_dir = Path::new(bin_manifest)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        for (dep_name, dep_path) in path_dependencies(&bin_text) {
            let lib_manifest = root.join(bin_dir).join(&dep_path).join("Cargo.toml");
            let Ok(lib_text) = fs::read_to_string(&lib_manifest) else {
                continue; 
            };
            if feature_declaration(&lib_text, QA_FEATURE).is_none() {
                continue; 
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
