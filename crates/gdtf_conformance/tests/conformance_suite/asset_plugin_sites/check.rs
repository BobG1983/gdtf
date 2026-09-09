//! Fail if any asset plugin outside the two hosts is built without the watcher turned off.

use std::path::Path;

use crate::asset_plugin_sites::{
    scan::{Verdict, verdict_for},
    tree::{repo_root, rust_files},
};

// The two real hosts. A running game and a running editor watch their asset root on
// purpose, so the rule below is theirs to break.
const HOSTS: [&str; 2] = [
    "crates/gdtf_game/src/app/gdtf_game.rs",
    "crates/gdtf_editor/src/app.rs",
];

fn offending_files(root: &Path, files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter(|path| !HOSTS.contains(&path.as_str()))
        .filter(|path| verdict_for(&root.join(path)) == Verdict::Watches)
        .cloned()
        .collect()
}

#[test]
fn every_asset_plugin_outside_the_hosts_turns_the_watcher_off() {
    let root = repo_root();
    assert!(
        root.join("crates").is_dir(),
        "no crates directory under {} — this guard is reading the wrong directory and \
         cannot see any source files",
        root.display()
    );
    let found = offending_files(&root, &rust_files(&root));
    assert!(
        found.is_empty(),
        "these files build an asset plugin that watches its asset root:\n{}\n\nSet \
         watch_for_changes_override: Some(false) on the plugin, or build it through \
         cobalt_test_utils::asset_plugin_at for a chosen root or \
         cobalt_test_utils::unwatched_asset_plugin for the default one. The watcher stats \
         every file under the root each time an app is built. The policy is in \
         docs/testing.md.",
        found.join("\n")
    );
}
