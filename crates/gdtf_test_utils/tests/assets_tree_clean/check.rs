use crate::git::{GitRead, git_lines, repo_root};

// Keeps untracked files and anything dirty in the worktree column. Staged-only
// entries pass: staging is deliberate intent heading into a gated commit.
fn unstaged_dirt(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let mut chars = line.chars();
            let index_col = chars.next().unwrap_or(' ');
            let worktree_col = chars.next().unwrap_or(' ');
            (index_col == '?' && worktree_col == '?') || worktree_col != ' '
        })
        .collect()
}

#[test]
fn tracked_assets_tree_is_clean() {
    let root = repo_root();

    match git_lines(&root, &["ls-files", "--", "assets/"]) {
        GitRead::Lines(tracked) => {
            assert!(
                !tracked.is_empty(),
                "git tracks no files under assets/ in {} — this guard is watching a tree that \
                 is not there, so it can never fail. If assets/ moved or was renamed, point \
                 the guard at the new path; otherwise restore the missing files.",
                root.display(),
            );
        }
        GitRead::Unavailable(reason) => {
            eprintln!(
                "assets-tree guard: tracked-file check skipped — {reason} (root: {})",
                root.display(),
            );
        }
    }

    match git_lines(&root, &["status", "--porcelain", "--", "assets/"]) {
        GitRead::Lines(lines) => {
            let dirty = unstaged_dirt(lines);
            for line in &dirty {
                eprintln!("{line}");
            }
            let rendered = dirty.join("\n");
            assert!(
                dirty.is_empty(),
                "the tracked assets/ tree has unstaged changes — a suite run must never mutate \
                 shipped authored content. Dirty entries:\n{rendered}\nIf these are YOUR \
                 authored-content edits, stage them (git add) or stash them — staged entries \
                 are deliberate intent and pass. If they are not yours, a test wrote into the \
                 tracked tree through a real save path — find that test, re-root it onto a \
                 TempDir, then restore the files with git checkout."
            );
        }
        GitRead::Unavailable(reason) => {
            eprintln!(
                "assets-tree guard: status check skipped — {reason} (root: {})",
                root.display(),
            );
        }
    }
}
