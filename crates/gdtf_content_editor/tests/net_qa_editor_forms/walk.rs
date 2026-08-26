use gdtf_battle_sim::{severity::Severity, terrain::def::TerrainTag};
use gdtf_content_editor::{EditorMode, TerrainKindChoice};

use crate::{
    names::EDITOR_SET_MODE,
    rows::FieldRow,
    setup::{
        editor_mode, form_tab_app_and_client, injury_draft, list_op, melee_weapon_draft, set_field,
        terrain_draft,
    },
    socket::run_editor,
    support::TestResult,
    values::{HandednessRow, SeverityRow},
};

#[test]
fn one_connection_walks_terrain_injury_and_melee_weapon_in_turn() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;

    let kind = set_field(&mut app, &mut client, "(field: Kind(Emplacement))")?;
    assert_eq!(
        kind.field,
        FieldRow::Kind(crate::values::TerrainKindRow::Emplacement)
    );
    assert_eq!(terrain_draft(&app)?.kind(), TerrainKindChoice::Emplacement);
    list_op(
        &mut app,
        &mut client,
        "(list: TerrainTags, op: Toggle(TerrainTag(BlocksVision)))",
    )?;
    assert!(terrain_draft(&app)?.has_tag(TerrainTag::BlocksVision));

    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Injury)"))?;
    assert_eq!(editor_mode(&app)?, EditorMode::Injury);
    let severity = set_field(&mut app, &mut client, "(field: InjurySeverity(Major))")?;
    assert_eq!(severity.field, FieldRow::InjurySeverity(SeverityRow::Major));
    assert_eq!(injury_draft(&app)?.def().severity, Severity::Major);
    list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_eq!(injury_draft(&app)?.effects().len(), 2);

    client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_MODE, "(mode: MeleeWeapon)"),
    )?;
    assert_eq!(editor_mode(&app)?, EditorMode::MeleeWeapon);
    let hands = set_field(
        &mut app,
        &mut client,
        "(field: MeleeWeaponHandedness(TwoHanded))",
    )?;
    assert_eq!(
        hands.field,
        FieldRow::MeleeWeaponHandedness(HandednessRow::TwoHanded)
    );
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Add)",
    )?;
    assert_eq!(melee_weapon_draft(&app)?.fight_modes().len(), 2);

    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "walking away from a tab leaves its draft where the earlier writes left it",
    );
    Ok(())
}
