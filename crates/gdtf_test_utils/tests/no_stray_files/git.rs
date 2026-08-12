use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) enum GitRead {
    Lines(Vec<String>),
    Unavailable(String),
}

// Git already answers this, so the guard never counts directory levels itself.
pub(crate) fn repo_root() -> GitReadRoot {
    match git_lines(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &["rev-parse", "--show-toplevel"],
    ) {
        GitRead::Lines(lines) => lines.first().map_or_else(
            || GitReadRoot::Unavailable("`git rev-parse --show-toplevel` printed nothing".into()),
            |line| GitReadRoot::Root(PathBuf::from(line)),
        ),
        GitRead::Unavailable(reason) => GitReadRoot::Unavailable(reason),
    }
}

pub(crate) enum GitReadRoot {
    Root(PathBuf),
    Unavailable(String),
}

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
