//! The Terrain inputs whose own range clamps a drag rather than refusing it.

use gdtf_editor::{EditorMode, TerrainDraft};

use crate::{
    mcp_editor_forms::{
        rows::{FieldRow, TerrainFieldRow},
        setup::{form_tab_app_and_client, set_field, terrain_draft},
    },
    mcp_shared::support::TestResult,
};

#[test]
fn an_hp_write_lands_on_both_cover_and_slab() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?;
    assert_ne!(
        (*before.cover_hp(), *before.slab_hp()),
        (250, 250),
        "the case must ask for an HP neither field already holds",
    );

    let written = set_field(&mut app, &mut client, "(field: Terrain(Hp(250)))")?;
    assert_eq!(written.field, FieldRow::Terrain(TerrainFieldRow::Hp(250)));
    let after = terrain_draft(&app)?;
    assert_eq!(
        (*after.cover_hp(), *after.slab_hp()),
        (250, 250),
        "the one HP input writes both the cover and the slab field, so writing one and not the \
         other fails here",
    );
    Ok(())
}

#[test]
fn an_hp_over_the_forms_range_is_stored_clamped_and_the_reply_says_so() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let ceiling = *TerrainDraft::HP_RANGE.end();

    let arguments = format!("(field: Terrain(Hp({})))", ceiling + 500);
    let written = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        written.field,
        FieldRow::Terrain(TerrainFieldRow::Hp(ceiling)),
        "the HP input's own range clamps a drag rather than refusing it, so the reply reads the \
         stored value back",
    );
    assert_eq!(*terrain_draft(&app)?.cover_hp(), ceiling);
    Ok(())
}

#[test]
fn an_armor_value_over_the_forms_range_is_stored_clamped_and_the_reply_says_so() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let ceiling = *TerrainDraft::ARMOR_RANGE.end();

    let arguments = format!("(field: Terrain(ArmorProtection({})))", ceiling + 50);
    let protection = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        protection.field,
        FieldRow::Terrain(TerrainFieldRow::ArmorProtection(ceiling)),
        "the armor input's own range clamps a drag rather than refusing it, so the reply reads \
         the stored value back",
    );
    assert_eq!(*terrain_draft(&app)?.armor_protection(), ceiling);

    let arguments = format!("(field: Terrain(ArmorHardness({})))", ceiling + 50);
    let hardness = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        hardness.field,
        FieldRow::Terrain(TerrainFieldRow::ArmorHardness(ceiling)),
        "the hardness input shares that range, so an over-range write reads back at the ceiling",
    );
    assert_eq!(*terrain_draft(&app)?.armor_hardness(), ceiling);
    Ok(())
}
