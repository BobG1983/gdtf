//! Fail if a doc or an agent rule cites a Rust file that is not on disk.

use std::{fs, path::Path};

use crate::tree::{SCANNED_ROOTS, repo_root, scanned_files};

// The three trees the workspace keeps Rust in. A cited path starts with one of them;
// a bare filename carries no directory, so nothing can resolve it and it is not read.
const CITED_AREAS: &[&str] = &["crates/", "libs/", "bins/"];

// The characters a cited path may hold, matching what the areas above name on disk.
const PATH_CHARS: &str = "_./-";

// A cited path and the file that cites it.
struct Citation {
    file: String,
    path: String,
}

#[test]
fn every_rust_path_cited_in_docs_or_claude_exists() {
    let found = repo_root();
    assert!(
        found.is_some(),
        "the guard found no workspace root above this crate, so it cannot resolve a cited path"
    );
    let Some(root) = found else {
        return;
    };

    let files = scanned_files(&root);
    for scanned in SCANNED_ROOTS {
        assert!(
            files
                .iter()
                .any(|file| file.starts_with(&format!("{scanned}/"))),
            "the scanned root {scanned} held no files under {} — this guard is reading the wrong \
             directory and would pass having read nothing",
            root.display()
        );
    }

    let cited = citations(&root, &files);
    assert!(
        !cited.is_empty(),
        "none of the {} files under {SCANNED_ROOTS:?} cited a backticked {CITED_AREAS:?} `.rs` \
         path — the scan found nothing to check and would pass on zero iterations",
        files.len()
    );

    let missing: Vec<String> = cited
        .iter()
        .filter(|citation| !root.join(&citation.path).exists())
        .map(|citation| format!("{} (cited in {})", citation.path, citation.file))
        .collect();

    assert!(
        missing.is_empty(),
        "these cited Rust paths are not on disk:\n{}\n\nMoving a test into a directory suite \
         leaves the old path in the prose that names it, where nothing but this guard reads it. \
         Correct each citation to the file's real path, or drop it.",
        missing.join("\n")
    );
}

// Every backticked Rust path in the scanned files. A file that does not read as UTF-8,
// such as a mockup image, is skipped rather than read.
fn citations(root: &Path, files: &[String]) -> Vec<Citation> {
    let mut found = Vec::new();
    for file in files {
        let Some(text) = fs::read_to_string(root.join(file)).ok() else {
            continue;
        };
        for line in text.lines() {
            // Splitting a line on its backticks puts the delimited spans at the odd indexes.
            for span in line
                .split('`')
                .skip(1)
                .step_by(2)
                .filter(|s| is_rust_path(s))
            {
                found.push(Citation {
                    file: file.clone(),
                    path: span.to_owned(),
                });
            }
        }
    }
    found
}

fn is_rust_path(span: &str) -> bool {
    CITED_AREAS.iter().any(|area| span.starts_with(area))
        && Path::new(span).extension().is_some_and(|ext| ext == "rs")
        && span
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || PATH_CHARS.contains(c))
}
