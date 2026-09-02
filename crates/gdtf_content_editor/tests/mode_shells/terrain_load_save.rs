//! Terrain mode: saving after a load writes the record that was loaded, not a second one.
#![cfg(debug_assertions)]

use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainView},
    facing::TerrainFacing,
    piece::TerrainGraphicKey,
};
use gdtf_content_editor::{TerrainDraft, TerrainKindChoice, write_terrain_in};

use crate::support::{advance_to_editing, editor_app, resolve_theme_display};

// The name the second draft carries before the load, so a skipped name shows as a second file.
const CONTRASTING_NAME: &str = "A Different Name Entirely";

#[test]
fn saving_a_loaded_terrain_overwrites_the_file_it_was_loaded_from() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    let theme_display = resolve_theme_display(&app);

    let Ok(root) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    let mut authored = TerrainDraft::default();
    authored.set_display_name("Load Save Probe".to_owned());
    authored.set_kind(TerrainKindChoice::Cover);
    authored.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new("cover".to_owned()),
    );
    let authored_key = authored.ensure_uuid();

    let Ok(first_path) = write_terrain_in(root.path(), &authored, authored_key, &theme_display)
    else {
        unreachable!("write_terrain_in must succeed for a named Cover draft into a TempDir")
    };
    let Ok(written) = std::fs::read_to_string(&first_path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(def) = ron::de::from_str::<TerrainDef>(&written) else {
        unreachable!("the written terrain def must parse through the loader's own parser")
    };

    let mut second = TerrainDraft::default();
    second.set_display_name(CONTRASTING_NAME.to_owned());
    let minted = second.ensure_uuid();
    assert_ne!(
        minted, def.key,
        "the second draft minted a key of its own, so the load has one to overwrite",
    );
    second.load_from_def(&def);

    let uuid = second.ensure_uuid();
    let Ok(second_path) = write_terrain_in(root.path(), &second, uuid, &theme_display) else {
        unreachable!("write_terrain_in must succeed for the loaded draft into the same TempDir")
    };

    assert_eq!(
        second_path, first_path,
        "the save after a load writes the loaded record's own file, because the load took the \
         def's display name and the path is built from it",
    );
    let Ok(rewritten) = std::fs::read_to_string(&second_path) else {
        unreachable!("the rewritten terrain def must be readable off disk")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&rewritten) else {
        unreachable!("the rewritten terrain def must parse through the loader's own parser")
    };
    assert_eq!(
        reloaded.key, def.key,
        "the save asks `ensure_uuid`, and after a load that answers the loaded key rather than \
         the one the second draft minted for itself",
    );

    let Some(folder) = first_path.parent() else {
        unreachable!("a written terrain def sits inside its theme folder")
    };
    let Ok(entries) = std::fs::read_dir(folder) else {
        unreachable!("the theme folder must be readable")
    };
    let files: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    assert_eq!(
        files.len(),
        1,
        "the two saves wrote one record between them, or the author now has two files under one \
         key: {files:?}",
    );
}
