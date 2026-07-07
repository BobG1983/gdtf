//! Git-state reads for the assets-tree cleanliness guard — the repo root and
//! the one `git` invocation shape both checks share (see the suite doc in
//! `main.rs`). Read-only: the guard never writes and never invokes cargo.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// The repo root — `GDTF_ASSETS_CLEAN_ROOT` override, else
/// `CARGO_MANIFEST_DIR/../..` (the sibling `module_layout` / `docs_path_truth`
/// guards' recipe).
pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_ASSETS_CLEAN_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// One git read, resolved: the non-empty stdout lines of a successful
/// invocation, or the reason git state is unreadable here.
pub(crate) enum GitRead {
    /// The invocation succeeded; these are its non-empty stdout lines
    /// (empty vec = the clean verdict the guard wants).
    Lines(Vec<String>),
    /// git is unavailable or `root` is not a git work tree — there is no
    /// baseline to compare against (the guard notes this and passes; the
    /// gate/CI context always runs inside a checkout).
    Unavailable(String),
}

/// Run `git -C <root> <args…>` and collect its non-empty stdout lines.
///
/// A spawn failure (no `git` binary) or a non-zero exit (not a work tree, an
/// unborn `HEAD`) resolves to [`GitRead::Unavailable`] carrying the reason —
/// the caller reports the skip honestly instead of fabricating a verdict.
pub(crate) fn git_lines(root: &Path, args: &[&str]) -> GitRead {
    let output = match Command::new("git").arg("-C").arg(root).args(args).output() {
        Ok(output) => output,
        Err(err) => return GitRead::Unavailable(format!("git failed to spawn: {err}")),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return GitRead::Unavailable(format!(
            "`git {}` exited non-zero ({}): {}",
            args.join(" "),
            output.status,
            stderr.trim(),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    GitRead::Lines(
        stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect(),
    )
}
