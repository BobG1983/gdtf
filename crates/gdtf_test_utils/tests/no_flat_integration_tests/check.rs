//! Fail if any crate or bin gains a new flat integration-test binary.

use crate::tree::{flat_test_files, repo_root};

// Flat test files already in the tree when the guard started walking all of it.
// They may stay; nothing new joins without a reason. Packing one into a directory
// suite or deleting it is fine — drop its line too.
const ALLOWED_FLATS: &[&str] = &[
    "crates/gdtf_game/tests/capstone.rs",
    "crates/gdtf_battle_presenter/tests/terrain_missing_sprite.rs",
    "libs/cobalt_mcp_protocol/tests/engine_free.rs",
    "libs/cobalt_mcp_server/tests/engine_free.rs",
    "libs/cobalt_mcp_server/tests/link_timeout.rs",
    "libs/cobalt_mcp_server/tests/loopback.rs",
    "libs/cobalt_mcp_server/tests/reconnect.rs",
];

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
         binary and links Bevy again, which makes the suite slow. Move it into a directory suite \
         — tests/<suite>/main.rs with the test in a module beside it — or, if it really has to \
         run on its own, add it to ALLOWED_FLATS in \
         crates/gdtf_test_utils/tests/no_flat_integration_tests/check.rs. The policy is in \
         docs/testing.md and .claude/rules/module-layout.md rule 5.",
        found.join("\n")
    );
}
