//! The Terrain form's own wrong-variant gate on an on-death row, addressed by its index.

use bevy::app::App;
use gdtf_battle_sim::effects::on_death::OnDeathEffect;
use gdtf_content_editor::EditorMode;

use crate::{
    outcome::unavailable_code,
    refusal::refusal_note,
    setup::{form_tab_app_and_client, list_op, set_field, terrain_draft, try_set_field},
    support::{TestError, TestResult},
};

// The effect the Terrain draft authors at one index, or why it authors none there.
fn effect_at(app: &App, index: usize) -> Result<OnDeathEffect, TestError> {
    match terrain_draft(app)?.on_death().get(index).cloned() {
        Some(held) => Ok(held),
        None => Err(format!("the Terrain draft authors no on-death effect at {index}").into()),
    }
}

#[test]
fn a_terrain_explode_payload_write_while_the_effect_leaves_a_field_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: Add)",
    )?;
    set_field(
        &mut app,
        &mut client,
        "(field: Terrain(OnDeathVariant(index: 0, variant: LeaveField)))",
    )?;
    let before = effect_at(&app, 0)?;

    for arguments in [
        "(field: Terrain(OnDeathHitType(index: 0, hit_type: Single)))",
        "(field: Terrain(OnDeathDamage(index: 0, damage: 12)))",
        "(field: Terrain(OnDeathDamageType(index: 0, damage_type: Blast)))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "the form draws `{arguments}` only while the row is Explode",
        );
        assert!(
            refusal_note(&reply)?.contains("LeaveField"),
            "the note names the variant the row is on",
        );
        assert_eq!(
            effect_at(&app, 0)?,
            before,
            "the refused write left the effect exactly as it was",
        );
    }
    Ok(())
}

#[test]
fn a_terrain_field_key_write_while_the_effect_explodes_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    list_op(
        &mut app,
        &mut client,
        "(list: TerrainOnDeathEffects, op: Add)",
    )?;
    let before = effect_at(&app, 0)?;

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(OnDeathField(index: 0, field: \"toxic_waste_pool\")))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form draws the field key only while the row is LeaveField",
    );
    assert!(
        refusal_note(&reply)?.contains("Explode"),
        "the note names the variant the row is on",
    );
    assert_eq!(
        effect_at(&app, 0)?,
        before,
        "the refused write left the effect exactly as it was",
    );
    Ok(())
}
