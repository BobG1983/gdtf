use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) const MEMBER_DIRS: [&str; 2] = ["crates", "bins"];

pub(crate) fn repo_root() -> PathBuf {
    if let Some(override_root) = std::env::var_os("GDTF_RUSTDOC_GATE_ROOT") {
        return PathBuf::from(override_root);
    }
    let Some(root) = gdtf_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn git_tracked(root: &Path, dir: &str) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", &format!("{dir}/*/Cargo.toml")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn walk_manifests(root: &Path, dir: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| format!("{dir}/{}/Cargo.toml", entry.file_name().to_string_lossy()))
        .filter(|path| root.join(path).is_file())
        .collect()
}

pub(crate) fn member_manifests(root: &Path) -> Vec<String> {
    let mut manifests: Vec<String> = MEMBER_DIRS
        .iter()
        .flat_map(|dir| git_tracked(root, dir).unwrap_or_else(|| walk_manifests(root, dir)))
        .filter(|path| path.matches('/').count() == 2)
        .collect();
    manifests.sort();
    manifests.dedup();
    manifests
}
