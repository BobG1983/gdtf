use std::{collections::BTreeSet, fs, path::Path};

use toml_edit::{DocumentMut, Item, Table};

use crate::{
    manifest::{item_at, lint_level, lint_priority, parse, strings},
    tree::{MEMBER_DIRS, member_manifests, repo_root},
};

const WORKSPACE_MANIFEST: &str = "Cargo.toml";

const RUSTDOC_GROUP: &str = "workspace.lints.rustdoc.all";

const DENY: &str = "deny";

// A negative priority lets the lints named under the group override the group level.
const GROUP_PRIORITY: i64 = -1;

const WORKSPACE_MEMBERS: &str = "workspace.members";

const LINTS_OPT_IN: &str = "lints.workspace";

const EDITOR_BIN_MANIFEST: &str = "bins/editor/Cargo.toml";

const EDITOR_BIN_NAME: &str = "editor";

const EDITOR_LIB_CRATE: &str = "crates/gdtf_editor";

fn read_manifest(root: &Path, path: &str) -> DocumentMut {
    let full = root.join(path);
    let Ok(text) = fs::read_to_string(&full) else {
        unreachable!(
            "cannot read {} — a manifest this guard cannot read is a broken guard, not a pass",
            full.display()
        );
    };
    parse(path, &text)
}

// The `[[bin]]` target with this name, if the manifest declares one.
fn bin_target<'a>(manifest: &'a DocumentMut, name: &str) -> Option<&'a Table> {
    manifest
        .get("bin")?
        .as_array_of_tables()?
        .iter()
        .find(|target| target.get("name").and_then(Item::as_str) == Some(name))
}

// Why this member entry is outside what `member_manifests` walks, or `None` when it is walked.
fn unwalked_member(member: &str) -> Option<String> {
    let Some(dir) = member.strip_suffix("/*").filter(|top| !top.contains('/')) else {
        return Some(format!(
            "`{member}` — this guard walks only members written `<dir>/*`, one directory deep, so \
             no crate this member names is checked. Adding to `MEMBER_DIRS` in `tree.rs` cannot \
             cover this shape; `member_manifests` there has to learn to walk it."
        ));
    };
    if MEMBER_DIRS.contains(&dir) {
        return None;
    }
    Some(format!(
        "`{member}` — no crate under `{dir}/` is checked. Add \"{dir}\" to `MEMBER_DIRS` in \
         `tree.rs`."
    ))
}

#[test]
fn workspace_denies_the_whole_rustdoc_lint_group() {
    let root = repo_root();
    let manifest = read_manifest(&root, WORKSPACE_MANIFEST);
    let group = item_at(&manifest, RUSTDOC_GROUP);
    let level = group.and_then(lint_level);
    let priority = group.and_then(lint_priority);
    let found_level = level.unwrap_or("nothing");
    let found_priority = priority
        .as_ref()
        .map_or_else(|| "nothing".to_owned(), ToString::to_string);
    assert!(
        level == Some(DENY) && priority == Some(GROUP_PRIORITY),
        "{WORKSPACE_MANIFEST} must set `{RUSTDOC_GROUP}` to level \"{DENY}\" with priority \
         {GROUP_PRIORITY} — without the group deny only the individually-named rustdoc lints can \
         fail a doc run, and every other rustdoc warning accumulates while \
         `cargo doc --workspace --no-deps` and `cargo doc-full` keep exiting 0. Any TOML spelling \
         works: inline table, dotted keys, either key order. It reads as level {found_level} and \
         priority {found_priority} today."
    );
}

#[test]
fn every_workspace_member_opts_in_to_the_workspace_lints() {
    let root = repo_root();
    let workspace = read_manifest(&root, WORKSPACE_MANIFEST);
    let members = item_at(&workspace, WORKSPACE_MEMBERS).map_or_else(Vec::new, strings);
    assert!(
        !members.is_empty(),
        "{WORKSPACE_MANIFEST} must list `{WORKSPACE_MEMBERS}` as strings — this guard reads that \
         list to know which crates it has to check for the `[lints] workspace = true` opt-in, and \
         a missing or empty list checks nothing"
    );

    let unwalked: Vec<String> = members
        .iter()
        .copied()
        .filter_map(unwalked_member)
        .collect();
    assert!(
        unwalked.is_empty(),
        "workspace members this guard does not walk, so the crates they name are never checked \
         for the `[lints] workspace = true` opt-in:\n{}",
        unwalked.join("\n")
    );

    let manifests = member_manifests(&root);
    assert!(
        !manifests.is_empty(),
        "no member Cargo.toml found under {:?} in {} — enumeration is broken",
        MEMBER_DIRS,
        root.display()
    );
    let mut violations: BTreeSet<String> = BTreeSet::new();
    for path in &manifests {
        let member = read_manifest(&root, path);
        if item_at(&member, LINTS_OPT_IN).and_then(Item::as_bool) != Some(true) {
            violations.insert(format!(
                "MISSING {path} — no `[lints]` table setting `workspace = true`; the workspace \
                 lint table, including the rustdoc group deny, does not reach this crate"
            ));
        }
    }
    for line in &violations {
        eprintln!("{line}");
    }
    let rendered = violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        violations.is_empty(),
        "workspace lint opt-in violations, across {} members:\n{rendered}",
        manifests.len()
    );
}

#[test]
fn the_content_editor_bin_is_excluded_from_the_doc_runs() {
    let root = repo_root();
    let manifest = read_manifest(&root, EDITOR_BIN_MANIFEST);
    let target = bin_target(&manifest, EDITOR_BIN_NAME);
    let excluded = target
        .and_then(|bin| bin.get("doc"))
        .and_then(Item::as_bool);
    let found = if target.is_none() {
        format!("there is no `[[bin]]` target named `{EDITOR_BIN_NAME}`")
    } else {
        excluded.map_or_else(
            || "that target sets no `doc` key".to_owned(),
            |value| format!("that target sets `doc = {value}`"),
        )
    };
    assert!(
        excluded == Some(false),
        "{EDITOR_BIN_MANIFEST} must set `doc = false` on its `[[bin]]` target named \
         `{EDITOR_BIN_NAME}` — that target and the library crate `{EDITOR_LIB_CRATE}` share the \
         name, so without it both write `target/doc/{EDITOR_BIN_NAME}/index.html`, one replaces \
         the other's rendered page, and a reader cannot tell which crate's docs they have \
         (rust-lang/cargo#6313). That warning comes from cargo, not rustdoc, so no lint level \
         catches it and both doc runs still exit 0. Today {found}."
    );
}
