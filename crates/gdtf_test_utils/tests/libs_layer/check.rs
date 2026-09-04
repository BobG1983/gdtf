//! Fail if a crate under `libs/` names a gdtf crate, or if the folder and the prefix disagree.

use std::{fs, path::Path};

use toml_edit::{DocumentMut, Item};

use crate::tree::{COBALT_PREFIX, GAME_DIRS, LIBS_DIR, member_manifests, repo_root};

// Every dependency table a `libs/` crate declares. A gdtf crate in any of them means the
// crate cannot be taken out of this repo.
const CONSUMER_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "dev-dependencies"];

const GDTF_PREFIX: &str = "gdtf_";

const RULE: &str = ".claude/rules/libs-layer.md";

fn read_manifest(root: &Path, path: &str) -> DocumentMut {
    let full = root.join(path);
    let Ok(text) = fs::read_to_string(&full) else {
        unreachable!(
            "cannot read {} — a manifest this guard cannot read is a broken guard, not a pass",
            full.display()
        );
    };
    match text.parse::<DocumentMut>() {
        Ok(manifest) => manifest,
        Err(error) => unreachable!("{path} is not valid TOML: {error}"),
    }
}

fn package_name(manifest: &DocumentMut) -> Option<&str> {
    manifest.get("package")?.get("name")?.as_str()
}

fn dependency_names<'a>(manifest: &'a DocumentMut, table: &str) -> Vec<&'a str> {
    manifest
        .get(table)
        .and_then(Item::as_table_like)
        .map_or_else(Vec::new, |deps| deps.iter().map(|(name, _)| name).collect())
}

#[test]
fn no_libs_crate_names_a_gdtf_crate_a_consumer_would_link() {
    let root = repo_root();
    let manifests = member_manifests(&root, LIBS_DIR);
    assert!(
        !manifests.is_empty(),
        "no member Cargo.toml found under {}/ in {} — enumeration is broken, and a guard that \
         checks nothing is not a pass",
        LIBS_DIR,
        root.display()
    );
    let mut violations: Vec<String> = Vec::new();
    for path in &manifests {
        let manifest = read_manifest(&root, path);
        for table in CONSUMER_TABLES {
            for name in dependency_names(&manifest, table) {
                if name.starts_with(GDTF_PREFIX) {
                    violations.push(format!("{path} — [{table}] names `{name}`"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "crates under {LIBS_DIR}/ are game-free, so nothing there may name a `{GDTF_PREFIX}` \
         crate under {CONSUMER_TABLES:?}:\n{}\n\nEither the dependency belongs in a gdtf crate \
         under crates/, or the thing being reached for is generic and belongs in a \
         `{COBALT_PREFIX}` crate of its own. The rule is {RULE}.",
        violations.join("\n")
    );
}

#[test]
fn the_libs_folder_and_the_cobalt_prefix_agree_in_both_directions() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();
    for path in member_manifests(&root, LIBS_DIR) {
        let manifest = read_manifest(&root, &path);
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
            let manifest = read_manifest(&root, &path);
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
         and nothing else carries it:\n{}\n\nThe rule is {RULE}.",
        violations.join("\n")
    );
}
