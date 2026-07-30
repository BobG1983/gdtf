//! The three rustdoc-gate tests — the group deny, its reach into every member,
//! and the content-editor doc exclusion (see the suite doc in `main.rs`).

use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    manifest::{declares, section_lines},
    tree::{MEMBER_DIRS, member_manifests, repo_root},
};

/// The group deny that makes every rustdoc lint fatal on both doc runs.
///
/// `priority = -1` is load-bearing, not decoration: without it an individual
/// lint listed below the group in the same table cannot override the group
/// level. The clippy table above it uses the same shape.
const GROUP_DENY: &str = r#"all = { level = "deny", priority = -1 }"#;

/// The section holding it.
const RUSTDOC_LINTS: &str = "workspace.lints.rustdoc";

/// The opt-in every member needs for `[workspace.lints]` to reach it at all.
const LINTS_OPT_IN: &str = "workspace = true";

/// The binary package whose bin target collided with the library crate's docs.
const EDITOR_BIN_MANIFEST: &str = "bins/gdtf_content_editor/Cargo.toml";

/// The exclusion that ends that collision.
const DOC_EXCLUSION: &str = "doc = false";

/// The section holding it — spelled with the inner brackets, so the reader
/// matches the array-of-tables header `[[bin]]` rather than a `[bin]` table.
const EDITOR_BIN_SECTION: &str = "[bin]";

/// Read a manifest under `root`, or fail the test naming the path — a manifest
/// this guard cannot read is a broken guard, not a pass.
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

/// The quoted entries of the `[workspace]` `members` array.
///
/// Handles the one-line and multi-line spellings alike by cutting the section
/// text at `members = [` and reading quoted entries up to the closing bracket.
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

/// The whole rustdoc lint group is denied, with the priority that keeps
/// individual overrides working.
#[test]
fn workspace_denies_the_whole_rustdoc_lint_group() {
    let root = repo_root();
    let manifest = read(&root, "Cargo.toml");
    let declared = section_lines(&manifest, RUSTDOC_LINTS).join("\n");
    assert!(
        declares(&manifest, RUSTDOC_LINTS, GROUP_DENY),
        "[{RUSTDOC_LINTS}] must declare `{GROUP_DENY}` (GTW-929) — without the group deny only \
         the individually-named lints can fail a doc run, and every other rustdoc warning \
         accumulates while `cargo doc --workspace --no-deps` and `cargo doc-full` keep exiting \
         0. Declared instead:\n{declared}"
    );
}

/// The deny reaches every member: a `[workspace.lints]` table applies only to a
/// crate that opts in, and the walk itself is pinned to the declared globs.
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
        "workspace lint opt-in violations (GTW-929), across {} members:\n{rendered}",
        manifests.len()
    );
}

/// The content-editor bin stays out of `cargo doc`, so its doc output cannot
/// collide with the library crate's again.
#[test]
fn the_content_editor_bin_is_excluded_from_the_doc_runs() {
    let root = repo_root();
    let manifest = read(&root, EDITOR_BIN_MANIFEST);
    let declared = section_lines(&manifest, EDITOR_BIN_SECTION).join("\n");
    assert!(
        declares(&manifest, EDITOR_BIN_SECTION, DOC_EXCLUSION),
        "{EDITOR_BIN_MANIFEST}'s `[[bin]]` must declare `{DOC_EXCLUSION}` (GTW-929) — this bin \
         target and the library crate `crates/gdtf_content_editor` share the name \
         `gdtf_content_editor`, so without the exclusion both write \
         `target/doc/gdtf_content_editor/index.html`, one replaces the other's rendered page, \
         and a reader cannot tell which crate's docs they have \
         (rust-lang/cargo#6313). That warning comes from cargo, not rustdoc, so no lint level \
         catches it and both doc runs still exit 0. Declared instead:\n{declared}"
    );
}
