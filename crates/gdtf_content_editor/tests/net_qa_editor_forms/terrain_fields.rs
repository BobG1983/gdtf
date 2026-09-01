use bevy::app::App;
use gdtf_assets::ContentFolderHandle;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    cover::HeightBand,
    effects::{fields::FieldKey, on_death::OnDeathEffect},
    terrain::def::LosBlocking,
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_content_editor::{EditorMode, FootfallChoice, TerrainKindChoice};
use gdtf_content_families::WeaponsFamily;

use crate::{
    names::EDITOR_SET_FIELD,
    outcome::unavailable_code,
    rows::{FieldRow, TerrainFieldRow},
    setup::{form_tab_app_and_client, list_op, set_field, terrain_draft, try_set_field},
    socket::{Client, run_editor},
    support::{TestError, TestResult},
    values::{
        BandRow, DamageTypeRow, FootfallRow, HitTypeRow, LosRow, OnDeathVariantRow, TerrainKindRow,
        TileRoleRow,
    },
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

    let body = set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    assert_eq!(
        body.field,
        FieldRow::Terrain(TerrainFieldRow::Kind(TerrainKindRow::Emplacement)),
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
        "(field: Terrain(DisplayName(\"bulkhead\")))",
    )?;
    assert_eq!(
        named.field,
        FieldRow::Terrain(TerrainFieldRow::DisplayName("bulkhead".to_owned())),
    );
    assert_eq!(terrain_draft(&app)?.display_name(), "bulkhead");

    let hp = set_field(&mut app, &mut client, "(field: Terrain(Hp(77)))")?;
    assert_eq!(hp.field, FieldRow::Terrain(TerrainFieldRow::Hp(77)));

    let protection = set_field(
        &mut app,
        &mut client,
        "(field: Terrain(ArmorProtection(9)))",
    )?;
    assert_eq!(
        protection.field,
        FieldRow::Terrain(TerrainFieldRow::ArmorProtection(9)),
    );
    assert_eq!(*terrain_draft(&app)?.armor_protection(), 9);

    let hardness = set_field(&mut app, &mut client, "(field: Terrain(ArmorHardness(8)))")?;
    assert_eq!(
        hardness.field,
        FieldRow::Terrain(TerrainFieldRow::ArmorHardness(8)),
    );
    assert_eq!(*terrain_draft(&app)?.armor_hardness(), 8);

    let graphic = set_field(&mut app, &mut client, "(field: Terrain(Graphic(Rubble)))")?;
    assert_eq!(
        graphic.field,
        FieldRow::Terrain(TerrainFieldRow::Graphic(TileRoleRow::Rubble)),
    );
    assert_eq!(terrain_draft(&app)?.graphic(), TileRole::Rubble);

    let pathing = set_field(
        &mut app,
        &mut client,
        "(field: Terrain(BlocksPathing(Some(true))))",
    )?;
    assert_eq!(
        pathing.field,
        FieldRow::Terrain(TerrainFieldRow::BlocksPathing(Some(true))),
    );
    assert_eq!(terrain_draft(&app)?.blocks_pathing(), Some(true));

    let los = set_field(
        &mut app,
        &mut client,
        "(field: Terrain(BlocksLos(Some(UpToHeightBand))))",
    )?;
    assert_eq!(
        los.field,
        FieldRow::Terrain(TerrainFieldRow::BlocksLos(Some(LosRow::UpToHeightBand))),
    );
    assert_eq!(
        terrain_draft(&app)?.blocks_los(),
        Some(LosBlocking::UpToHeightBand)
    );

    set_field(&mut app, &mut client, "(field: Terrain(Kind(Slab)))")?;
    let footfall = set_field(&mut app, &mut client, "(field: Terrain(Footfall(Grate)))")?;
    assert_eq!(
        footfall.field,
        FieldRow::Terrain(TerrainFieldRow::Footfall(FootfallRow::Grate)),
    );
    assert_eq!(terrain_draft(&app)?.footfall(), FootfallChoice::Grate);

    set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    let band = set_field(&mut app, &mut client, "(field: Terrain(HeightBand(Low)))")?;
    assert_eq!(
        band.field,
        FieldRow::Terrain(TerrainFieldRow::HeightBand(BandRow::Low)),
    );
    assert_eq!(terrain_draft(&app)?.height_band(), HeightBand::Low);

    let weapon = a_mounted_weapon(&app)?;
    let arguments = format!(
        "(field: Terrain(MountedWeapon(Some(\"{}\"))))",
        weapon.as_str()
    );
    let mounted = set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        mounted.field,
        FieldRow::Terrain(TerrainFieldRow::MountedWeapon(Some(
            weapon.as_str().to_owned()
        ))),
    );
    assert_eq!(terrain_draft(&app)?.mounted_weapon(), Some(&weapon));

    every_on_death_arm(&mut app, &mut client)
}

// The five on-death arms, each written at the one index an Add put there.
fn every_on_death_arm(app: &mut App, client: &mut Client) -> TestResult {
    list_op(app, client, "(list: TerrainOnDeathEffects, op: Add)")?;
    let geometry = set_field(
        app,
        client,
        "(field: Terrain(OnDeathHitType(index: 0, hit_type: Blast(radius: 2))))",
    )?;
    assert_eq!(
        geometry.field,
        FieldRow::Terrain(TerrainFieldRow::OnDeathHitType {
            index:    0,
            hit_type: HitTypeRow::Blast { radius: 2 },
        }),
    );
    let damage = set_field(
        app,
        client,
        "(field: Terrain(OnDeathDamage(index: 0, damage: 14)))",
    )?;
    assert_eq!(
        damage.field,
        FieldRow::Terrain(TerrainFieldRow::OnDeathDamage {
            index:  0,
            damage: 14,
        }),
    );
    let channel = set_field(
        app,
        client,
        "(field: Terrain(OnDeathDamageType(index: 0, damage_type: Blast)))",
    )?;
    assert_eq!(
        channel.field,
        FieldRow::Terrain(TerrainFieldRow::OnDeathDamageType {
            index:       0,
            damage_type: DamageTypeRow::Blast,
        }),
    );
    let picked = set_field(
        app,
        client,
        "(field: Terrain(OnDeathVariant(index: 0, variant: LeaveField)))",
    )?;
    assert_eq!(
        picked.field,
        FieldRow::Terrain(TerrainFieldRow::OnDeathVariant {
            index:   0,
            variant: OnDeathVariantRow::LeaveField,
        }),
    );
    let keyed = set_field(
        app,
        client,
        "(field: Terrain(OnDeathField(index: 0, field: \"toxic_waste_pool\")))",
    )?;
    assert_eq!(
        keyed.field,
        FieldRow::Terrain(TerrainFieldRow::OnDeathField {
            index: 0,
            field: "toxic_waste_pool".to_owned(),
        }),
    );
    assert_eq!(
        terrain_draft(app)?.on_death(),
        [OnDeathEffect::LeaveField {
            field: FieldKey::new("toxic_waste_pool".to_owned()),
        }],
        "the draft holds the row every write above left it on",
    );
    Ok(())
}

#[test]
fn a_graphic_role_the_picker_does_not_offer_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?.graphic();

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(Graphic(Door)))")?;
    crate::bad_arguments::bad_arguments_detail(&reply)?;
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

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
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
        run_editor(EDITOR_SET_FIELD, "(field: Terrain(Kind(Emplacement)))"),
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
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    let before = terrain_draft(&app)?.mounted_weapon().cloned();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(MountedWeapon(Some(\"no_such_weapon\"))))",
    )?;
    crate::bad_arguments::bad_arguments_detail(&reply)?;
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
    set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    app.world_mut().remove_resource::<WeaponRegistry>();
    // The content family rebuilds a registry that left the world while its folder handle is held.
    app.world_mut()
        .remove_resource::<ContentFolderHandle<WeaponsFamily>>();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(MountedWeapon(Some(\"autogun\"))))",
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
