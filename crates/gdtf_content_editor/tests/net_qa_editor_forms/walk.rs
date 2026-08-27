use gdtf_battle_sim::{armor::ArmorType, severity::Severity, terrain::def::TerrainTag};
use gdtf_content_editor::{EditorMode, TerrainKindChoice};

use crate::{
    names::{EDITOR_NEW, EDITOR_SET_MODE},
    rows::{FieldFormFieldRow, FieldRow, InjuryFieldRow, MeleeWeaponFieldRow, TerrainFieldRow},
    setup::{
        editor_mode, field_draft, form_tab_app_and_client, injury_draft, list_op,
        melee_weapon_draft, set_field, terrain_draft,
    },
    socket::run_editor,
    support::TestResult,
    values::{HandednessRow, SeverityRow},
};

#[test]
fn one_connection_walks_terrain_injury_and_melee_weapon_in_turn() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;

    let kind = set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    assert_eq!(
        kind.field,
        FieldRow::Terrain(TerrainFieldRow::Kind(
            crate::values::TerrainKindRow::Emplacement
        ))
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
    let severity = set_field(&mut app, &mut client, "(field: Injury(Severity(Major)))")?;
    assert_eq!(
        severity.field,
        FieldRow::Injury(InjuryFieldRow::Severity(SeverityRow::Major)),
    );
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
        "(field: MeleeWeapon(Handedness(TwoHanded)))",
    )?;
    assert_eq!(
        hands.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Handedness(HandednessRow::TwoHanded)),
    );
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Add)",
    )?;
    assert_eq!(melee_weapon_draft(&app)?.fight_modes().len(), 2);

    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Field)"))?;
    assert_eq!(editor_mode(&app)?, EditorMode::Field);
    client.exchange(&mut app, &run_editor(EDITOR_NEW, "(mode: Field)"))?;
    let damage = set_field(&mut app, &mut client, "(field: Field(Damage(6)))")?;
    assert_eq!(damage.field, FieldRow::Field(FieldFormFieldRow::Damage(6)));
    list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))",
    )?;
    assert!(field_draft(&app)?.is_immune(ArmorType::Flak));

    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Terrain)"))?;
    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "walking away from a tab leaves its draft where the earlier writes left it",
    );
    assert_eq!(
        *field_draft(&app)?.damage(),
        6,
        "walking off the Field tab leaves its own writes in place too",
    );
    Ok(())
}
