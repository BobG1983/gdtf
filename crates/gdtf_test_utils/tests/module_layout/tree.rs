use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn workspace_root() -> PathBuf {
    if let Some(override_root) = std::env::var_os("GDTF_MODULE_LAYOUT_ROOT") {
        return PathBuf::from(override_root);
    }
    let Some(root) = cobalt_ron_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "crates", "bins", "libs"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn walk_files(dir: &Path, root: &Path, acc: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, root, acc);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let parts: Vec<_> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            acc.push(parts.join("/"));
        }
    }
}

pub(crate) fn tracked_rs(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| {
        let mut acc = Vec::new();
        walk_files(&root.join("crates"), root, &mut acc);
        walk_files(&root.join("bins"), root, &mut acc);
        walk_files(&root.join("libs"), root, &mut acc);
        acc
    });
    files.retain(|p| Path::new(p).extension().is_some_and(|e| e == "rs"));
    files.sort();
    files
}
