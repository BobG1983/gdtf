use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_DOCS_PATH_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "docs", ".claude/rules"])
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
        if entry.file_name().to_string_lossy() == "target" {
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

pub(crate) fn scanned_docs(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| {
        let mut acc = Vec::new();
        walk_files(&root.join("docs"), root, &mut acc);
        walk_files(&root.join(".claude/rules"), root, &mut acc);
        acc
    });
    files.retain(|p| {
        Path::new(p)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("txt"))
    });
    files.sort();
    files
}
