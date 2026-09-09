//! Terrain mode: saving after a load writes the record that was loaded, not a second one.

use std::path::{Path, PathBuf};

use gdtf_assets::ContentSourcePath;
use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainView},
    facing::TerrainFacing,
    piece::TerrainGraphicKey,
};
use gdtf_editor::{TerrainDraft, TerrainKindChoice, write_terrain_in};

use crate::mode_shells::support::{advance_to_editing, editor_app, resolve_theme_display};

// The name the second draft carries before the load, so a skipped name shows as a second file.
const CONTRASTING_NAME: &str = "A Different Name Entirely";

// A named Cover draft with one view filled, which is the least `write_terrain_in` accepts.
fn probe_draft(name: &str) -> TerrainDraft {
    let mut draft = TerrainDraft::default();
    draft.set_display_name(name.to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new("cover".to_owned()),
    );
    draft
}

// Every file sitting in the theme folder a save wrote into.
fn files_beside(path: &Path) -> Vec<PathBuf> {
    let Some(folder) = path.parent() else {
        unreachable!("a written terrain def sits inside its theme folder")
    };
    let Ok(entries) = std::fs::read_dir(folder) else {
        unreachable!("the theme folder must be readable")
    };
    entries.flatten().map(|entry| entry.path()).collect()
}

// The def a written file holds, read back through the loader's own parser.
fn parse_written(path: &Path) -> TerrainDef {
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(def) = ron::de::from_str::<TerrainDef>(&text) else {
        unreachable!("the written terrain def must parse through the loader's own parser")
    };
    def
}

#[test]
fn saving_a_loaded_terrain_overwrites_the_file_it_was_loaded_from() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    let theme_display = resolve_theme_display(&app);

    let Ok(root) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    let mut authored = probe_draft("Load Save Probe");
    let authored_key = authored.ensure_uuid();

    let Ok(first_path) = write_terrain_in(root.path(), &authored, authored_key, &theme_display)
    else {
        unreachable!("write_terrain_in must succeed for a named Cover draft into a TempDir")
    };
    let def = parse_written(&first_path);

    let mut second = TerrainDraft::default();
    second.set_display_name(CONTRASTING_NAME.to_owned());
    let minted = second.ensure_uuid();
    assert_ne!(
        minted, def.key,
        "the second draft minted a key of its own, so the load has one to overwrite",
    );
    second.load_from_def(&def, None);

    let uuid = second.ensure_uuid();
    let Ok(second_path) = write_terrain_in(root.path(), &second, uuid, &theme_display) else {
        unreachable!("write_terrain_in must succeed for the loaded draft into the same TempDir")
    };

    assert_eq!(
        second_path, first_path,
        "a draft carrying no source path rebuilds one from its display name and theme, and after \
         a load those still name the file the def came from",
    );
    assert_eq!(
        parse_written(&second_path).key,
        def.key,
        "the save asks `ensure_uuid`, and after a load that answers the loaded key rather than \
         the one the second draft minted for itself",
    );

    let files = files_beside(&first_path);
    assert_eq!(
        files.len(),
        1,
        "the two saves wrote one record between them, or the author now has two files under one \
         key: {files:?}",
    );
}

#[test]
fn renaming_a_loaded_terrain_saves_over_the_file_it_was_loaded_from() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    let theme_display = resolve_theme_display(&app);
    let other_theme = format!("{theme_display} Annex");
    assert_ne!(
        other_theme, theme_display,
        "the second save names a different theme, or it cannot show the folder is ignored",
    );

    let Ok(root) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    let mut authored = probe_draft("Rename Probe");
    let authored_key = authored.ensure_uuid();
    let Ok(first_path) = write_terrain_in(root.path(), &authored, authored_key, &theme_display)
    else {
        unreachable!("write_terrain_in must succeed for a named Cover draft into a TempDir")
    };
    let def = parse_written(&first_path);
    let Ok(relative) = first_path.strip_prefix(root.path()) else {
        unreachable!("the first save landed under the TempDir it was handed")
    };

    let mut opened = TerrainDraft::default();
    opened.load_from_def(&def, Some(ContentSourcePath::new(relative.to_path_buf())));
    opened.set_display_name(CONTRASTING_NAME.to_owned());

    let uuid = opened.ensure_uuid();
    let Ok(second_path) = write_terrain_in(root.path(), &opened, uuid, &other_theme) else {
        unreachable!("write_terrain_in must succeed for the renamed loaded draft")
    };

    assert_eq!(
        second_path, first_path,
        "the save writes the file the def was loaded from, whatever the draft is now named and \
         whatever theme the session is on",
    );
    let files = files_beside(&first_path);
    assert_eq!(
        files.len(),
        1,
        "the rename rewrote the loaded record, or the author now has two files under one key: \
         {files:?}",
    );
    let reloaded = parse_written(&second_path);
    assert_eq!(
        reloaded.key, def.key,
        "the rewritten file still carries the loaded def's key, so the rename renamed a record \
         rather than minting one",
    );
    assert_eq!(
        *reloaded.display_name, CONTRASTING_NAME,
        "the rewritten file carries the new name; a save that wrote nothing leaves the loaded \
         name on disk and passes every other assertion here",
    );
}
