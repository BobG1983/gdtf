use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    manifest::{declares, section_lines},
    tree::{MEMBER_DIRS, member_manifests, repo_root},
};

const GROUP_DENY: &str = r#"all = { level = "deny", priority = -1 }"#;

const RUSTDOC_LINTS: &str = "workspace.lints.rustdoc";

const LINTS_OPT_IN: &str = "workspace = true";

const EDITOR_BIN_MANIFEST: &str = "bins/gdtf_content_editor/Cargo.toml";

const DOC_EXCLUSION: &str = "doc = false";

const EDITOR_BIN_SECTION: &str = "[bin]";

fn read(root: &Path, path: &str) -> String {
    let full = root.join(path);
    let text = fs::read_to_string(&full);
    assert!(
        text.is_ok(),
        "cannot read {} — a manifest this guard cannot read is a broken guard, not a pass",
        full.display()
    );
    text.unwrap_or_default()
}

fn member_globs(manifest: &str) -> Vec<String> {
    let section = section_lines(manifest, "workspace").join(" ");
    let Some((_, after)) = section.split_once("members = [") else {
        return Vec::new();
    };
    let array = after.split_once(']').map_or(after, |(inside, _)| inside);
    array
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn workspace_denies_the_whole_rustdoc_lint_group() {
    let root = repo_root();
    let manifest = read(&root, "Cargo.toml");
    let declared = section_lines(&manifest, RUSTDOC_LINTS).join("\n");
    assert!(
        declares(&manifest, RUSTDOC_LINTS, GROUP_DENY),
        "[{RUSTDOC_LINTS}] must declare `{GROUP_DENY}` — without the group deny only \
         the individually-named lints can fail a doc run, and every other rustdoc warning \
         accumulates while `cargo doc --workspace --no-deps` and `cargo doc-full` keep exiting \
         0. Declared instead:\n{declared}"
    );
}

#[test]
fn every_workspace_member_opts_in_to_the_workspace_lints() {
    let root = repo_root();
    let workspace = read(&root, "Cargo.toml");
    let globs = member_globs(&workspace);
    let expected: Vec<String> = MEMBER_DIRS.iter().map(|dir| format!("{dir}/*")).collect();
    assert_eq!(
        globs, expected,
        "the workspace `members` globs changed — this guard walks only {expected:?}, so a member \
         outside them would never be checked for its `[lints] workspace = true` opt-in. Update \
         `MEMBER_DIRS` in `tree.rs` alongside the manifest."
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
        let member = read(&root, path);
        if !declares(&member, "lints", LINTS_OPT_IN) {
            violations.insert(format!(
                "MISSING {path} — no `[lints]` section declaring `{LINTS_OPT_IN}`; the workspace \
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
    let manifest = read(&root, EDITOR_BIN_MANIFEST);
    let declared = section_lines(&manifest, EDITOR_BIN_SECTION).join("\n");
    assert!(
        declares(&manifest, EDITOR_BIN_SECTION, DOC_EXCLUSION),
        "{EDITOR_BIN_MANIFEST}'s `[[bin]]` must declare `{DOC_EXCLUSION}` — this bin \
         target and the library crate `crates/gdtf_content_editor` share the name \
         `gdtf_content_editor`, so without the exclusion both write \
         `target/doc/gdtf_content_editor/index.html`, one replaces the other's rendered page, \
         and a reader cannot tell which crate's docs they have \
         (rust-lang/cargo#6313). That warning comes from cargo, not rustdoc, so no lint level \
         catches it and both doc runs still exit 0. Declared instead:\n{declared}"
    );
}
