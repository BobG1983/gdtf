use std::path::Path;

use crate::git::{GitRead, GitReadRoot, git_lines, repo_root};

fn assert_git_tracks_something(root: &Path) {
    match git_lines(root, &["ls-files"]) {
        GitRead::Lines(tracked) => {
            assert!(
                !tracked.is_empty(),
                "git tracks no files at all in {} — this guard is watching a tree that is not \
                 there, so it can never fail. Point it at the real checkout.",
                root.display(),
            );
        }
        GitRead::Unavailable(reason) => {
            eprintln!(
                "stray-file guard: tracked-file check skipped — {reason} (root: {})",
                root.display(),
            );
        }
    }
}

fn assert_no_untracked_files(root: &Path) {
    match git_lines(root, &["ls-files", "-o", "--exclude-standard"]) {
        GitRead::Lines(stray) => {
            for line in &stray {
                eprintln!("{line}");
            }
            let rendered = stray.join("\n");
            assert!(
                stray.is_empty(),
                "the repo holds untracked files — a suite run must write nothing into the \
                 checkout. Untracked paths:\n{rendered}\nIf these are YOURS, stage them (git \
                 add) — staged files pass. If they are not, a test wrote into the tree; re-root \
                 that test onto a TempDir and delete the files.",
            );
        }
        GitRead::Unavailable(reason) => {
            eprintln!(
                "stray-file guard: untracked check skipped — {reason} (root: {})",
                root.display(),
            );
        }
    }
}

#[test]
fn the_repo_holds_no_untracked_files() {
    match repo_root() {
        GitReadRoot::Root(root) => {
            assert_git_tracks_something(&root);
            assert_no_untracked_files(&root);
        }
        GitReadRoot::Unavailable(reason) => {
            eprintln!("stray-file guard: skipped — {reason}");
        }
    }
}
