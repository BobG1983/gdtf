//! Fail if a `libs/` crate reaches outside `libs/`, or if the folder and the prefix disagree.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use toml_edit::{DocumentMut, Item};

use crate::tree::{COBALT_PREFIX, GAME_DIRS, LIBS_DIR, member_manifests, repo_root};

// Every dependency table a `libs/` crate declares. A path leaving `libs/` in any of them
// means the crate cannot be taken out of this repo.
const CONSUMER_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "dev-dependencies"];

const RULE: &str = ".claude/rules/libs-layer.md";

// The manifest at `root/path`, or a finding naming that path and why it could not be had. A
// manifest this guard cannot read is a broken guard, so the caller reports it and walks on.
pub(crate) fn read_manifest(root: &Path, path: &str) -> Result<DocumentMut, String> {
    let text = match fs::read_to_string(root.join(path)) {
        Ok(text) => text,
        Err(error) => return Err(format!("{path} — cannot be read: {error}")),
    };
    text.parse::<DocumentMut>()
        .map_err(|error| format!("{path} — is not valid TOML: {error}"))
}

fn package_name(manifest: &DocumentMut) -> Option<&str> {
    manifest.get("package")?.get("name")?.as_str()
}

// Every `(name, entry)` pair in one dependency table.
fn dependency_entries<'a>(manifest: &'a DocumentMut, table: &str) -> Vec<(&'a str, &'a Item)> {
    manifest
        .get(table)
        .and_then(Item::as_table_like)
        .map_or_else(Vec::new, |deps| deps.iter().collect())
}

// The `path = "…"` an entry declares, if it declares one.
fn declared_path(entry: &Item) -> Option<&str> {
    entry.get("path")?.as_str()
}

// Whether an entry defers to the root `[workspace.dependencies]` table.
fn inherits_from_workspace(entry: &Item) -> bool {
    entry
        .get("workspace")
        .and_then(Item::as_bool)
        .unwrap_or(false)
}

// `base` joined with `relative`, with `.` and `..` resolved by text.
fn resolve(base: &Path, relative: &str) -> PathBuf {
    let mut resolved = base.to_path_buf();
    for part in Path::new(relative).components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    resolved
}

#[test]
fn no_libs_crate_path_dependency_leaves_libs() {
    let root = repo_root();
    let libs = root.join(LIBS_DIR);
    let mut violations: Vec<String> = Vec::new();
    let workspace = match read_manifest(&root, "Cargo.toml") {
        Ok(manifest) => Some(manifest),
        Err(finding) => {
            violations.push(finding);
            None
        }
    };
    let inherited = workspace.as_ref().and_then(|manifest| {
        manifest
            .get("workspace")
            .and_then(|table| table.get("dependencies"))
    });
    let manifests = member_manifests(&root, LIBS_DIR);
    assert!(
        !manifests.is_empty(),
        "no member Cargo.toml found under {}/ in {} — enumeration is broken, and a guard that \
         checks nothing is not a pass. Findings so far:\n{}",
        LIBS_DIR,
        root.display(),
        violations.join("\n")
    );
    for path in &manifests {
        let manifest = match read_manifest(&root, path) {
            Ok(manifest) => manifest,
            Err(finding) => {
                violations.push(finding);
                continue;
            }
        };
        let crate_dir = resolve(&root, path)
            .parent()
            .map_or_else(|| root.clone(), Path::to_path_buf);
        for table in CONSUMER_TABLES {
            for (name, entry) in dependency_entries(&manifest, table) {
                let (base, declared) = if let Some(declared) = declared_path(entry) {
                    (crate_dir.clone(), declared)
                } else if inherits_from_workspace(entry) {
                    let Some(declared) = inherited
                        .and_then(|deps| deps.get(name))
                        .and_then(declared_path)
                    else {
                        continue;
                    };
                    (root.clone(), declared)
                } else {
                    continue;
                };
                let target = resolve(&base, declared);
                if !target.starts_with(&libs) {
                    violations.push(format!(
                        "{path} — [{table}] `{name}` has path `{declared}`, which resolves to {} \
                         outside {}/",
                        target.display(),
                        LIBS_DIR
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a crate under {LIBS_DIR}/ must build in a project that holds nothing else, so no \
         dependency of it under {CONSUMER_TABLES:?} — declared inline or inherited from the root \
         `[workspace.dependencies]` — may point at a path outside {LIBS_DIR}/. A manifest this \
         guard could not read or parse is a failure too, because a manifest it skipped is one it \
         did not check:\n{}\n\nEither the dependency belongs in a gdtf crate under crates/, or \
         the thing being reached for is generic and belongs in a `{COBALT_PREFIX}` crate of its \
         own. The rule is {RULE}.",
        violations.join("\n")
    );
}

#[test]
fn the_libs_folder_and_the_cobalt_prefix_agree_in_both_directions() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();
    for path in member_manifests(&root, LIBS_DIR) {
        let manifest = match read_manifest(&root, &path) {
            Ok(manifest) => manifest,
            Err(finding) => {
                violations.push(finding);
                continue;
            }
        };
        let name = package_name(&manifest).unwrap_or("");
        if !name.starts_with(COBALT_PREFIX) {
            violations.push(format!(
                "{path} — package `{name}` sits under {LIBS_DIR}/ without the `{COBALT_PREFIX}` \
                 prefix"
            ));
        }
    }
    for dir in GAME_DIRS {
        for path in member_manifests(&root, dir) {
            let manifest = match read_manifest(&root, &path) {
                Ok(manifest) => manifest,
                Err(finding) => {
                    violations.push(finding);
                    continue;
                }
            };
            let name = package_name(&manifest).unwrap_or("");
            if name.starts_with(COBALT_PREFIX) {
                violations.push(format!(
                    "{path} — package `{name}` carries the `{COBALT_PREFIX}` prefix but sits \
                     under {dir}/"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the `{COBALT_PREFIX}` prefix means the crate is game-free and lives under {LIBS_DIR}/, \
         and nothing else carries it. A manifest this guard could not read or parse is a failure \
         too, because a manifest it skipped is one it did not check:\n{}\n\nThe rule is {RULE}.",
        violations.join("\n")
    );
}
