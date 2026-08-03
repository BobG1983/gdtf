use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_CI_WORKFLOW_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", ".github/workflows"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn walk_files(root: &Path) -> Vec<String> {
    let dir = root.join(".github/workflows");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| format!(".github/workflows/{}", entry.file_name().to_string_lossy()))
        .collect()
}

pub(crate) fn workflow_files(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| walk_files(root));
    files.retain(|p| {
        Path::new(p)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
    });
    files.sort();
    files
}
