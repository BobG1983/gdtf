//! Fail if a packed crate gains a new flat integration-test binary.

use std::{collections::BTreeSet, fs};

use crate::tree::{PACKED_TEST_CRATES, repo_root};

/// Flat files intentionally left solo (process-global or special harness).
const ALLOWED_FLATS: &[(&str, &str)] = &[
    ("crates/gdtf_app", "capstone.rs"),
    ("crates/gdtf_battle_presenter", "terrain_missing_sprite.rs"),
];

#[test]
fn packed_crates_have_no_unexpected_flat_integration_tests() {
    let root = repo_root();
    let allowed: BTreeSet<(&str, &str)> = ALLOWED_FLATS.iter().copied().collect();
    let mut violations = Vec::new();

    for crate_rel in PACKED_TEST_CRATES {
        let tests = root.join(crate_rel).join("tests");
        if !tests.is_dir() {
            violations.push(format!("missing tests dir: {crate_rel}/tests"));
            continue;
        }
        let Ok(entries) = fs::read_dir(&tests) else {
            violations.push(format!("unreadable: {crate_rel}/tests"));
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if allowed.contains(&(crate_rel, name)) {
                continue;
            }
            violations.push(format!(
                "{crate_rel}/tests/{name} — flat integration binary; pack into a dir suite \
                 (see docs/tooling/test-pack.md). Solo exception requires allowlist update."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "unexpected flat integration tests:\n{}",
        violations.join("\n")
    );
}

#[test]
fn allowlisted_solo_flats_still_exist() {
    let root = repo_root();
    for (crate_rel, name) in ALLOWED_FLATS {
        let path = root.join(crate_rel).join("tests").join(name);
        assert!(
            path.is_file(),
            "allowlisted solo flat missing: {} — update ALLOWED_FLATS if removed",
            path.display()
        );
    }
}
