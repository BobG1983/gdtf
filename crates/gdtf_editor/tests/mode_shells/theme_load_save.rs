//! Theme mode: saving after a rename rewrites the file the theme was loaded from.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

use std::path::{Path, PathBuf};

use bevy::asset::uuid::Uuid;
use gdtf_assets::{ContentFamily, ContentSourcePath};
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeDef},
    terrain::def::TerrainUuid,
};
use gdtf_content_families::ThemeDefsFamily;
use gdtf_editor::{ThemeDraft, write_theme_in};

// The name the loaded draft is renamed to, whose slug names a different folder.
const RENAMED: &str = "A Different Name Entirely";

// A named theme holding one terrain as its own default floor, which is what a save accepts.
fn probe_draft(name: &str) -> ThemeDraft {
    let floor = TerrainUuid::new(Uuid::from_u128(0x0184_0bcd_9001));
    ThemeDraft::from_parts(
        ThemeUuid::new(Uuid::from_u128(0x0184_0bcd_9101)),
        name.to_owned(),
        vec![floor],
        floor,
    )
}

// The def a written file holds, read back through the loader's own parser.
fn parse_written(path: &Path) -> UuidThemeDef {
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!("the written theme def must be readable off disk inside the TempDir")
    };
    let Ok(def) = ron::de::from_str::<UuidThemeDef>(&text) else {
        unreachable!("the written theme def must parse through the loader's own parser")
    };
    def
}

// Every theme file in the family folder, counted across its subfolders too.
fn theme_files_under(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        unreachable!("the theme family folder must be readable: {folder:?}")
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(theme_files_under(&path));
        } else if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(ThemeDefsFamily::EXTENSION))
        {
            found.push(path);
        }
    }
    found
}

#[test]
fn renaming_a_loaded_theme_saves_over_the_file_it_was_loaded_from() {
    let Ok(root) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    let authored = probe_draft("Theme Rename Probe");
    let Ok(first_path) = write_theme_in(root.path(), &authored, authored.key()) else {
        unreachable!("write_theme_in must succeed for a named theme draft into a TempDir")
    };
    let def = parse_written(&first_path);
    assert_ne!(
        RENAMED, *def.display_name,
        "the rename must change the name, or a save that mints its path lands on the same file",
    );
    let Ok(relative) = first_path.strip_prefix(root.path()) else {
        unreachable!("the first save landed under the TempDir it was handed")
    };

    let mut opened = ThemeDraft::from_parts(
        def.key,
        (*def.display_name).clone(),
        def.terrain.clone(),
        def.default_floor,
    );
    opened.set_source(Some(ContentSourcePath::new(relative.to_path_buf())));
    opened.set_display_name(RENAMED.to_owned());

    let Ok(second_path) = write_theme_in(root.path(), &opened, opened.key()) else {
        unreachable!("write_theme_in must succeed for the renamed loaded draft")
    };

    assert_eq!(
        second_path, first_path,
        "the save writes the file the def was loaded from, whatever the draft is now named",
    );
    let files = theme_files_under(&root.path().join(ThemeDefsFamily::FOLDER));
    assert_eq!(
        files.len(),
        1,
        "the rename rewrote the loaded record, or the author now has two files under one key — \
         the second one a folder along: {files:?}",
    );
    let reloaded = parse_written(&second_path);
    assert_eq!(
        reloaded.key,
        authored.key(),
        "the rewritten file still carries the loaded def's key, so the rename renamed a record \
         rather than minting one",
    );
    assert_eq!(
        *reloaded.display_name, RENAMED,
        "the rewritten file carries the new name; a save that wrote nothing leaves the loaded \
         name on disk and passes every other assertion here",
    );
}
