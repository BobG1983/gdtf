use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_ASSETS_CLEAN_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

pub(crate) enum GitRead {
            Lines(Vec<String>),
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
