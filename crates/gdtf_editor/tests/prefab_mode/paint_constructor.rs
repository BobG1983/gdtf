//! The two paint paths build their placement through the session, not by hand.

use std::{fs, path::PathBuf};

// The two paths that may only reach a placement through `MapEditorSession::paint_proposal`.
const PAINT_PATHS: [&str; 2] = ["src/egui_shell/prefab", "src/mcp/commands/write/paint"];

// The constructor those paths may not call, because it takes a facing of its own.
const BANNED: &str = "ProposedPlacement::new";

// Every `.rs` file under `dir`, skipping any path holding a test directory segment.
fn rust_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut pending = vec![dir];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_test_dir = path
                .file_name()
                .is_some_and(|name| name == "test" || name == "tests");
            if path.is_dir() {
                if !is_test_dir {
                    pending.push(path);
                }
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn no_paint_path_builds_a_placement_with_its_own_facing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut hits: Vec<String> = Vec::new();
    let mut read: usize = 0;
    for path in PAINT_PATHS {
        let files = rust_files(root.join(path));
        assert!(
            !files.is_empty(),
            "`{path}` holds the paint entry point this case scans, and the walk found no .rs \
             file there, so the assertion below would pass against anything",
        );
        for file in files {
            read += 1;
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for (number, line) in text.lines().enumerate() {
                if line.contains(BANNED) && !line.trim_start().starts_with("//") {
                    hits.push(format!("{}:{}", file.display(), number + 1));
                }
            }
        }
    }
    hits.sort();
    assert!(
        read >= PAINT_PATHS.len(),
        "the scan must read at least one file per path, and it read {read}",
    );
    assert!(
        hits.is_empty(),
        "the canvas click and editor.paint both build their placement through \
         MapEditorSession::paint_proposal, so the author's chosen facing reaches the piece; \
         these call `{BANNED}` instead: {hits:?}",
    );
}
