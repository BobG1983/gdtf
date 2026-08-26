use bevy::app::App;
use gdtf_assets::ContentFolderHandle;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    cover::HeightBand,
    terrain::def::LosBlocking,
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_content_editor::{EditorMode, FootfallChoice, TerrainDraft, TerrainKindChoice};
use gdtf_content_families::WeaponsFamily;

use crate::{
    names::EDITOR_SET_FIELD,
    outcome::unavailable_code,
    rows::FieldRow,
    setup::{form_tab_app_and_client, set_field, terrain_draft, try_set_field},
    socket::run_editor,
    support::{TestError, TestResult},
    values::{BandRow, FootfallRow, LosRow, TerrainKindRow, TileRoleRow},
};

/// The first weapon the mounted-weapon picker offers, read off the registry the form reads.
pub(crate) fn a_mounted_weapon(app: &App) -> Result<WeaponName, TestError> {
    let Some(registry) = app.world().get_resource::<WeaponRegistry>() else {
        return Err("the weapon registry is what the mounted-weapon picker reads".into());
    };
    let mut names: Vec<&WeaponName> = registry.keys().collect();
    names.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    match names.first() {
        Some(first) => Ok((*first).clone()),
        None => Err("the weapon registry holds no key, so no mounted weapon can be picked".into()),
    }
}

#[test]
fn set_field_writes_the_terrain_kind_the_reply_and_the_world_agree() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    assert_ne!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "the case must ask for a kind the draft is not already on, or it would pass against a \
         handler that writes nothing",
    );

    let body = set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    assert_eq!(
        body.field,
        FieldRow::Kind(TerrainKindRow::Emplacement),
        "the reply names the field that was written, read back off the draft",
    );
    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "the world's own Terrain draft is on the requested kind on the frame that answered. A \
         reply-only assertion would pass against a handler that writes nothing",
    );
    Ok(())
}

#[test]
fn every_terrain_field_writes_the_draft_and_reads_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;

    let named = set_field(
        &mut app,
        &mut client,
        "(field: TerrainDisplayName(\"bulkhead\"))",
    )?;
    assert_eq!(
        named.field,
        FieldRow::TerrainDisplayName("bulkhead".to_owned())
    );
    assert_eq!(terrain_draft(&app)?.display_name(), "bulkhead");

    let hp = set_field(&mut app, &mut client, "(field: TerrainHp(77))")?;
    assert_eq!(hp.field, FieldRow::TerrainHp(77));

    let protection = set_field(&mut app, &mut client, "(field: TerrainArmorProtection(9))")?;
    assert_eq!(protection.field, FieldRow::TerrainArmorProtection(9));
    assert_eq!(*terrain_draft(&app)?.armor_protection(), 9);

    let hardness = set_field(&mut app, &mut client, "(field: TerrainArmorHardness(8))")?;
    assert_eq!(hardness.field, FieldRow::TerrainArmorHardness(8));
    assert_eq!(*terrain_draft(&app)?.armor_hardness(), 8);

    let graphic = set_field(&mut app, &mut client, "(field: TerrainGraphic(Rubble))")?;
    assert_eq!(graphic.field, FieldRow::TerrainGraphic(TileRoleRow::Rubble));
    assert_eq!(terrain_draft(&app)?.graphic(), TileRole::Rubble);

    let pathing = set_field(
        &mut app,
        &mut client,
        "(field: TerrainBlocksPathing(Some(true)))",
    )?;
    assert_eq!(pathing.field, FieldRow::TerrainBlocksPathing(Some(true)));
    assert_eq!(terrain_draft(&app)?.blocks_pathing(), Some(true));

    let los = set_field(
        &mut app,
        &mut client,
        "(field: TerrainBlocksLos(Some(UpToHeightBand)))",
    )?;
    assert_eq!(
        los.field,
        FieldRow::TerrainBlocksLos(Some(LosRow::UpToHeightBand))
    );
    assert_eq!(
        terrain_draft(&app)?.blocks_los(),
        Some(LosBlocking::UpToHeightBand)
    );

    set_field(&mut app, &mut client, "(field: Kind(Slab))")?;
    let footfall = set_field(&mut app, &mut client, "(field: TerrainFootfall(Grate))")?;
    assert_eq!(
        footfall.field,
        FieldRow::TerrainFootfall(FootfallRow::Grate)
    );
    assert_eq!(terrain_draft(&app)?.footfall(), FootfallChoice::Grate);

    set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    let band = set_field(&mut app, &mut client, "(field: TerrainHeightBand(Low))")?;
    assert_eq!(band.field, FieldRow::TerrainHeightBand(BandRow::Low));
    assert_eq!(terrain_draft(&app)?.height_band(), HeightBand::Low);

    let weapon = a_mounted_weapon(&app)?;
    let arguments = format!(
        "(field: TerrainMountedWeapon(Some(\"{}\")))",
        weapon.as_str()
    );
    let mounted = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        mounted.field,
        FieldRow::TerrainMountedWeapon(Some(weapon.as_str().to_owned())),
    );
    assert_eq!(terrain_draft(&app)?.mounted_weapon(), Some(&weapon));
    Ok(())
}

#[test]
fn an_hp_write_lands_on_both_cover_and_slab() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?;
    assert_ne!(
        (*before.cover_hp(), *before.slab_hp()),
        (250, 250),
        "the case must ask for an HP neither field already holds",
    );

    let written = set_field(&mut app, &mut client, "(field: TerrainHp(250))")?;
    assert_eq!(written.field, FieldRow::TerrainHp(250));
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

    let arguments = format!("(field: TerrainHp({}))", ceiling + 500);
    let written = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        written.field,
        FieldRow::TerrainHp(ceiling),
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

    let arguments = format!("(field: TerrainArmorProtection({}))", ceiling + 50);
    let protection = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        protection.field,
        FieldRow::TerrainArmorProtection(ceiling),
        "the armor input's own range clamps a drag rather than refusing it, so the reply reads \
         the stored value back",
    );
    assert_eq!(*terrain_draft(&app)?.armor_protection(), ceiling);

    let arguments = format!("(field: TerrainArmorHardness({}))", ceiling + 50);
    let hardness = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        hardness.field,
        FieldRow::TerrainArmorHardness(ceiling),
        "the hardness input shares that range, so an over-range write reads back at the ceiling",
    );
    assert_eq!(*terrain_draft(&app)?.armor_hardness(), ceiling);
    Ok(())
}

#[test]
fn a_graphic_role_the_picker_does_not_offer_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?.graphic();

    let reply = try_set_field(&mut app, &mut client, "(field: TerrainGraphic(Door))")?;
    crate::refusal::bad_arguments_detail(&reply)?;
    assert_eq!(
        terrain_draft(&app)?.graphic(),
        before,
        "the refused write left the graphic role as it was",
    );
    Ok(())
}

#[test]
fn set_field_is_refused_on_the_prefab_tab() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Prefab)?;

    let reply = try_set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "this command writes the open form's draft, and the Prefab tab is the map canvas and \
         holds no draft, so it is refused rather than written through",
    );
    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::default(),
        "a refused write leaves the Terrain draft exactly as it was",
    );
    Ok(())
}

#[test]
fn set_field_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = crate::load_case::reply_answered_during_load(
        run_editor(EDITOR_SET_FIELD, "(field: Kind(Emplacement))"),
        "the editor.set_field run",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft is state-scoped to Editing, so a Load-pass call is refused rather than \
         creating one",
    );
    Ok(())
}

#[test]
fn a_mounted_weapon_key_no_registry_holds_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    let before = terrain_draft(&app)?.mounted_weapon().cloned();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: TerrainMountedWeapon(Some(\"no_such_weapon\")))",
    )?;
    crate::refusal::bad_arguments_detail(&reply)?;
    assert_eq!(
        terrain_draft(&app)?.mounted_weapon().cloned(),
        before,
        "the picker offers no such row, so the refused write mounted nothing",
    );
    Ok(())
}

#[test]
fn a_mounted_weapon_write_with_no_registry_is_missing_model() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    app.world_mut().remove_resource::<WeaponRegistry>();
    // The content family rebuilds a registry that left the world while its folder handle is held.
    app.world_mut()
        .remove_resource::<ContentFolderHandle<WeaponsFamily>>();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: TerrainMountedWeapon(Some(\"autogun\")))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "the picker reads the weapon registry, and with none in the world the form offers no \
         name at all, which is a missing model rather than a wrong state",
    );
    assert_eq!(
        terrain_draft(&app)?.mounted_weapon(),
        None,
        "the refused write mounted nothing",
    );
    Ok(())
}
