//! The cleanliness test — both git reads must come back empty, and every
//! dirty line is named loudly in the failure (see the suite doc in `main.rs`).

use crate::git::{GitRead, git_lines, repo_root};

/// The GTW-653 assets-tree cleanliness guard (see the suite doc in `main.rs`):
/// `git status --porcelain -- assets/` AND `git diff HEAD --stat -- assets/`
/// must both be empty at the guard's run time. Catches the PERSISTENT-mutation
/// incident class (a test wrote through a real save path and the dirt survived
/// the run) — a same-run transient write-then-revert is per-test isolation's
/// job (the C1 audit), not this guard's.
#[test]
fn tracked_assets_tree_is_clean() {
    let root = repo_root();

    // Worktree + index changes and untracked files under assets/.
    let status = git_lines(&root, &["status", "--porcelain", "--", "assets/"]);
    // The stronger HEAD-relative diff — catches staged-over dirt from a prior run.
    let diff = git_lines(&root, &["diff", "HEAD", "--stat", "--", "assets/"]);

    let mut dirty: Vec<String> = Vec::new();
    for (label, read) in [("STATUS", status), ("DIFF-HEAD", diff)] {
        match read {
            GitRead::Lines(lines) => {
                dirty.extend(lines.into_iter().map(|line| format!("{label} {line}")));
            }
            GitRead::Unavailable(reason) => {
                // No git baseline here (no binary / not a work tree / unborn
                // HEAD) — note the skip honestly and keep going; the gate and
                // CI always run inside a checkout, where this branch is dead.
                eprintln!(
                    "assets-tree guard: {label} check skipped — {reason} (root: {})",
                    root.display(),
                );
            }
        }
    }

    for line in &dirty {
        eprintln!("{line}");
    }
    let rendered = dirty.join("\n");
    assert!(
        dirty.is_empty(),
        "the tracked assets/ tree is DIRTY (assets-tree cleanliness guard, GTW-653) — a suite \
         run must never mutate shipped authored content. Dirty entries:\n{rendered}\nIf these \
         are YOUR authored-content edits, commit or stash them before running the suite; if \
         not, a test wrote into the tracked tree through a real save path — find it and re-root \
         it onto a TempDir (the GTW-555/636 pattern), then restore the files via git checkout."
    );
}
