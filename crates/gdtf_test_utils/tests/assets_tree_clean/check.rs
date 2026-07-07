//! The cleanliness test — both git reads must come back empty of UNSTAGED
//! dirt, and every dirty line is named loudly in the failure (see the suite
//! doc in `main.rs`).

use crate::git::{GitRead, git_lines, repo_root};

/// Keeps only the porcelain lines that are UNSTAGED/UNTRACKED dirt: untracked
/// entries (`??`) and entries whose WORKTREE column (the second status char)
/// is set. Staged-only entries (index column set, worktree column a space —
/// e.g. `A ` / `M `) are deliberate author/orchestrator intent en route to a
/// commit and pass: tests never run `git add`, so the incident class this
/// guard exists for (a test writing through a real save path) can only ever
/// produce unstaged or untracked dirt (the staged-is-intent semantics, the
/// GTW-663 catch-22 fix).
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

/// The GTW-653 assets-tree cleanliness guard (see the suite doc in `main.rs`):
/// `git status --porcelain -- assets/` must show no UNSTAGED/UNTRACKED entries
/// and `git diff --stat -- assets/` (worktree vs index — the unstaged diff)
/// must be empty at the guard's run time. Catches the PERSISTENT-mutation
/// incident class (a test wrote through a real save path and the dirt survived
/// the run) — a same-run transient write-then-revert is per-test isolation's
/// job (the C1 audit), and STAGED entries are deliberate intent heading into a
/// gated commit (GTW-663), not this guard's business.
#[test]
fn tracked_assets_tree_is_clean() {
    let root = repo_root();

    // Worktree + index changes and untracked files under assets/; staged-only
    // entries are filtered out below (staged-is-intent, GTW-663).
    let status = git_lines(&root, &["status", "--porcelain", "--", "assets/"]);
    // The worktree-vs-index diff — UNSTAGED modifications to tracked files.
    let diff = git_lines(&root, &["diff", "--stat", "--", "assets/"]);

    let mut dirty: Vec<String> = Vec::new();
    for (label, read) in [("STATUS", status), ("DIFF-UNSTAGED", diff)] {
        match read {
            GitRead::Lines(lines) => {
                let lines = if label == "STATUS" {
                    unstaged_dirt(lines)
                } else {
                    lines
                };
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
        "the tracked assets/ tree has UNSTAGED dirt (assets-tree cleanliness guard, GTW-653) — \
         a suite run must never mutate shipped authored content. Dirty entries:\n{rendered}\nIf \
         these are YOUR authored-content edits, stage (git add) or stash them — staged entries \
         are deliberate intent and pass (GTW-663); if they are not yours, a test wrote into the \
         tracked tree through a real save path — find it and re-root it onto a TempDir (the \
         GTW-555/636 pattern), then restore the files via git checkout."
    );
}
