use crate::git::{GitRead, git_lines, repo_root};

/// e.g. `A ` / `M `) are deliberate author/orchestrator intent en route to a
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

    let status = git_lines(&root, &["status", "--porcelain", "--", "assets/"]);
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
        "the tracked assets/ tree has UNSTAGED dirt (assets-tree cleanliness guard) — \
         a suite run must never mutate shipped authored content. Dirty entries:\n{rendered}\nIf \
         these are YOUR authored-content edits, stage (git add) or stash them — staged entries \
         are deliberate intent and pass; if they are not yours, a test wrote into the \
         tracked tree through a real save path — find it and re-root it onto a TempDir (the \
         /636 pattern), then restore the files via git checkout."
    );
}
