use gdtf_battle_sim::terrain::def::LosBlocking;
use gdtf_content_editor::EditorMode;

use crate::{
    rows::FieldRow,
    setup::{form_tab_app_and_client, set_field, terrain_draft},
    support::TestResult,
    terrain_fields::a_mounted_weapon,
};

#[test]
fn a_none_mounted_weapon_write_unmounts_the_weapon() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    let weapon = a_mounted_weapon(&app)?;
    let arguments = format!(
        "(field: TerrainMountedWeapon(Some(\"{}\")))",
        weapon.as_str()
    );
    set_field(&mut app, &mut client, &arguments)?;
    assert_eq!(
        terrain_draft(&app)?.mounted_weapon(),
        Some(&weapon),
        "the clear has to start from a mounted weapon, or it would pass against a handler that \
         writes nothing",
    );

    let cleared = set_field(&mut app, &mut client, "(field: TerrainMountedWeapon(None))")?;
    assert_eq!(
        cleared.field,
        FieldRow::TerrainMountedWeapon(None),
        "the reply reads the mount back off the draft, so a clear that did not land answers the \
         weapon that is still mounted",
    );
    assert_eq!(
        terrain_draft(&app)?.mounted_weapon(),
        None,
        "a None write unmounts the weapon, the way clearing the picker does",
    );
    Ok(())
}

#[test]
fn a_none_blocks_pathing_write_returns_the_field_to_the_kinds_own_value() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(
        &mut app,
        &mut client,
        "(field: TerrainBlocksPathing(Some(true)))",
    )?;
    assert_eq!(
        terrain_draft(&app)?.blocks_pathing(),
        Some(true),
        "the clear has to start from an override, or it would pass against a handler that writes \
         nothing",
    );

    let cleared = set_field(&mut app, &mut client, "(field: TerrainBlocksPathing(None))")?;
    assert_eq!(
        cleared.field,
        FieldRow::TerrainBlocksPathing(None),
        "the reply reads the override back off the draft, so a clear stored as Some(false) \
         answers Some(false) here",
    );
    assert_eq!(
        terrain_draft(&app)?.blocks_pathing(),
        None,
        "a None write drops the override, which is not the same as overriding it to false",
    );
    Ok(())
}

#[test]
fn a_none_blocks_los_write_returns_the_field_to_the_kinds_own_value() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(
        &mut app,
        &mut client,
        "(field: TerrainBlocksLos(Some(UpToHeightBand)))",
    )?;
    assert_eq!(
        terrain_draft(&app)?.blocks_los(),
        Some(LosBlocking::UpToHeightBand),
        "the clear has to start from an override, or it would pass against a handler that writes \
         nothing",
    );

    let cleared = set_field(&mut app, &mut client, "(field: TerrainBlocksLos(None))")?;
    assert_eq!(
        cleared.field,
        FieldRow::TerrainBlocksLos(None),
        "the reply reads the override back off the draft, so a clear stored as a blocking value \
         answers that value here",
    );
    assert_eq!(
        terrain_draft(&app)?.blocks_los(),
        None,
        "a None write drops the override, which is not the same as overriding it to Never",
    );
    Ok(())
}
