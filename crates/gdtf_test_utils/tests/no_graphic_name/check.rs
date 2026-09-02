//! Fail if a content file, a test fixture or a doc still names `graphic_name`.

use std::fs;

use crate::tree::{SCANNED_ROOTS, repo_root, scanned_files};

// The terrain-def field that was removed. A leftover inside a `Slab(...)` block,
// inside a comment, or inside a doc paragraph parses and loads without complaint,
// so nothing but this guard catches it.
const RETIRED_FIELD: &str = "graphic_name";

#[test]
fn no_content_fixture_or_doc_still_names_graphic_name() {
    let root = repo_root();
    assert!(
        root.join("assets/content").is_dir(),
        "no assets/content directory under {} — this guard is reading the wrong directory and \
         cannot see any content files",
        root.display()
    );
    let files = scanned_files(&root);
    assert!(
        !files.is_empty(),
        "the scanned roots {SCANNED_ROOTS:?} held no files under {} — this guard would pass on \
         zero iterations",
        root.display()
    );

    let holding: Vec<String> = files
        .into_iter()
        .filter(|path| {
            fs::read_to_string(root.join(path)).is_ok_and(|text| text.contains(RETIRED_FIELD))
        })
        .collect();

    assert!(
        holding.is_empty(),
        "these files still name the retired terrain-def field `{RETIRED_FIELD}`:\n{}\n\nA \
         `.terrain_def.ron` names its art through its `views:` rows now, and a `Slab(...)` block \
         keeps only `footfall`. A leftover key inside a Slab block, a leftover mention in a \
         fixture comment, and a leftover doc paragraph all load and read fine, so this guard is \
         the only thing that reports them.",
        holding.join("\n")
    );
}
