use gdtf_editor::EditorMode;

use crate::{
    mcp_editor_forms::{
        refusal::refusal_note,
        setup::{form_tab_app_and_client, set_field, terrain_draft, try_list_op, try_set_field},
    },
    mcp_shared::{outcome::unavailable_code, support::TestResult},
};

#[test]
fn a_footfall_write_off_slab_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Wall)))")?;
    let before = terrain_draft(&app)?.footfall();

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(Footfall(Metal)))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form draws the footfall pick only on a Slab, and the draft's own setter silently \
         does nothing otherwise, so the write is refused",
    );
    assert!(
        refusal_note(&reply)?.contains("footfall"),
        "the note names the gate that is closed",
    );
    assert_eq!(
        terrain_draft(&app)?.footfall(),
        before,
        "the refused write left the footfall the kind change had already settled",
    );
    Ok(())
}

#[test]
fn a_mounted_weapon_write_off_emplacement_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Cover)))")?;
    let before = terrain_draft(&app)?.mounted_weapon().cloned();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(MountedWeapon(Some(\"autogun\"))))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft commits a mounted weapon only on an Emplacement, so a Cover draft is refused \
         rather than written through a setter that would silently do nothing",
    );
    assert!(
        refusal_note(&reply)?.contains("Emplacement"),
        "the note names the gate that is closed",
    );
    assert_eq!(
        terrain_draft(&app)?.mounted_weapon().cloned(),
        before,
        "the refused write left the mounted weapon the kind change had already settled",
    );
    Ok(())
}

#[test]
fn a_height_band_write_off_a_banded_kind_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Slab)))")?;
    let before = terrain_draft(&app)?.height_band();

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(HeightBand(Low)))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form hides the height band on a Slab, and the draft's own setter gates nothing, so \
         the handler is what has to refuse it",
    );
    assert!(
        refusal_note(&reply)?.contains("height band"),
        "the note names the gate that is closed",
    );
    assert_eq!(
        terrain_draft(&app)?.height_band(),
        before,
        "the refused write left the height band exactly as it was",
    );
    Ok(())
}

#[test]
fn a_view_write_off_a_kind_that_does_not_owe_it_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Wall)))")?;
    let before = terrain_draft(&app)?.views().clone();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(View(view: Facing(North), sprite: \"wall\")))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "an untagged Wall owes its edges and its corners and no Facing view at all, so the \
         handler refuses the write rather than storing art nothing will ever draw",
    );
    assert!(
        refusal_note(&reply)?.contains("Facing(North)"),
        "the note names the view the draft does not owe",
    );
    assert_eq!(
        terrain_draft(&app)?.views().clone(),
        before,
        "the refused write left the draft's views exactly as they were",
    );
    Ok(())
}

#[test]
fn an_entry_side_toggle_off_emplacement_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    crate::mcp_editor_forms::setup::list_op(
        &mut app,
        &mut client,
        "(list: EntrySides, op: Toggle(EntrySide(East)))",
    )?;
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Wall)))")?;
    let before = terrain_draft(&app)?.entry_sides().to_vec();

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: EntrySides, op: Toggle(EntrySide(East)))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft commits entry sides only on an Emplacement kind, so a Wall draft is refused \
         rather than written through a setter that would silently do nothing",
    );
    assert_eq!(
        terrain_draft(&app)?.entry_sides().to_vec(),
        before,
        "the refused write left the sides the kind change had already settled",
    );
    Ok(())
}
