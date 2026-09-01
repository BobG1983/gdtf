//! The Weapon form's on-death list, its variant combo and each variant's own payload.

use bevy::app::App;
use gdtf_battle_sim::{
    effects::on_death::{ExplodeDamage, OnDeathEffect},
    weapon::{BlastRadius, DamageType, HitType},
};
use gdtf_content_editor::EditorMode;

use crate::{
    bad_arguments::bad_arguments_detail,
    outcome::unavailable_code,
    refusal::refusal_note,
    rows::{FieldRow, WeaponFieldRow},
    setup::{form_tab_app_and_client, list_op, set_field, try_set_field, weapon_draft},
    support::{TestError, TestResult},
    values::{DamageTypeRow, HitTypeRow, OnDeathVariantRow},
};

// The effect the draft authors at one index, or why it authors none there.
fn effect_at(app: &App, index: usize) -> Result<OnDeathEffect, TestError> {
    match weapon_draft(app)?.spec().on_death.get(index).cloned() {
        Some(held) => Ok(held),
        None => Err(format!("the Weapon draft authors no on-death effect at {index}").into()),
    }
}

// How many effects the draft's on-death list holds.
fn held(app: &App) -> Result<usize, TestError> {
    Ok(weapon_draft(app)?.spec().on_death.len())
}

#[test]
fn an_add_seeds_the_forms_own_explode_template_and_its_rows_then_write_it() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert_eq!(
        held(&app)?,
        0,
        "a fresh Weapon draft authors no on-death effect, or the Add would prove nothing",
    );

    let added = list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;
    assert_eq!(added.members.len(), 1, "Add appends one row");
    assert_eq!(
        effect_at(&app, 0)?,
        OnDeathEffect::Explode {
            hit_type:    HitType::Single,
            damage:      ExplodeDamage::new(0),
            damage_type: DamageType::Kinetic,
        },
        "the Add seeds the same blank explode the form's own Add button seeds",
    );

    let geometry = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathHitType(index: 0, hit_type: Blast(radius: 2))))",
    )?;
    assert_eq!(
        geometry.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathHitType {
            index:    0,
            hit_type: HitTypeRow::Blast { radius: 2 },
        }),
    );
    let damage = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathDamage(index: 0, damage: 12)))",
    )?;
    assert_eq!(
        damage.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathDamage {
            index:  0,
            damage: 12,
        }),
    );
    let channel = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathDamageType(index: 0, damage_type: Blast)))",
    )?;
    assert_eq!(
        channel.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathDamageType {
            index:       0,
            damage_type: DamageTypeRow::Blast,
        }),
    );

    let OnDeathEffect::Explode {
        hit_type,
        damage,
        damage_type,
    } = effect_at(&app, 0)?
    else {
        return Err("the three payload writes left the row on the other variant".into());
    };
    assert_eq!(
        hit_type,
        HitType::Blast {
            radius: BlastRadius::new(2),
        }
    );
    assert_eq!(*damage, 12);
    assert_eq!(damage_type, DamageType::Blast);
    Ok(())
}

#[test]
fn the_variant_pick_swaps_the_template_and_the_field_key_then_writes() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;

    let picked = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(index: 0, variant: LeaveField)))",
    )?;
    assert_eq!(
        picked.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathVariant {
            index:   0,
            variant: OnDeathVariantRow::LeaveField,
        }),
    );

    let keyed = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathField(index: 0, field: \"promethium_pool\")))",
    )?;
    assert_eq!(
        keyed.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathField {
            index: 0,
            field: "promethium_pool".to_owned(),
        }),
    );
    let OnDeathEffect::LeaveField { field } = effect_at(&app, 0)? else {
        return Err("the variant pick left the row on Explode".into());
    };
    assert_eq!(field.as_str(), "promethium_pool");

    let back = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(index: 0, variant: Explode)))",
    )?;
    assert_eq!(
        back.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathVariant {
            index:   0,
            variant: OnDeathVariantRow::Explode,
        }),
    );
    assert!(
        matches!(effect_at(&app, 0)?, OnDeathEffect::Explode { .. }),
        "picking the other variant swaps in that variant's blank template",
    );
    Ok(())
}

#[test]
fn an_on_death_payload_write_against_an_empty_list_is_refused_and_seeds_no_effect() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;

    for arguments in [
        "(field: Weapon(OnDeathVariant(index: 0, variant: LeaveField)))",
        "(field: Weapon(OnDeathHitType(index: 0, hit_type: Single)))",
        "(field: Weapon(OnDeathDamage(index: 0, damage: 12)))",
        "(field: Weapon(OnDeathDamageType(index: 0, damage_type: Blast)))",
        "(field: Weapon(OnDeathField(index: 0, field: \"promethium_pool\")))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        let detail = bad_arguments_detail(&reply)?;
        assert!(
            detail.contains('0') && detail.contains("holds 0"),
            "`{arguments}` names index 0 of an empty list, so the detail names the index and \
             the length, got `{detail}`",
        );
        assert_eq!(
            held(&app)?,
            0,
            "the refused write must not seed an effect as a side effect",
        );
    }
    Ok(())
}

#[test]
fn an_on_death_payload_write_past_the_end_of_the_list_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;
    let before = weapon_draft(&app)?.spec().on_death.clone();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathDamage(index: 1, damage: 12)))",
    )?;

    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains('1') && detail.contains("holds 1"),
        "a write at index 1 against a list holding one names the index and the length, got \
         `{detail}`",
    );
    assert_eq!(
        weapon_draft(&app)?.spec().on_death,
        before,
        "an index past the end writes nothing, so a handler that clamps to the last row fails \
         here",
    );
    Ok(())
}

#[test]
fn an_explode_payload_write_while_the_effect_leaves_a_field_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;
    set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(index: 0, variant: LeaveField)))",
    )?;
    let before = effect_at(&app, 0)?;

    for arguments in [
        "(field: Weapon(OnDeathHitType(index: 0, hit_type: Single)))",
        "(field: Weapon(OnDeathDamage(index: 0, damage: 12)))",
        "(field: Weapon(OnDeathDamageType(index: 0, damage_type: Blast)))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "the form draws `{arguments}` only while the effect is Explode",
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
fn a_field_key_write_while_the_effect_explodes_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeaponOnDeathEffects, op: Add)",
    )?;
    let before = effect_at(&app, 0)?;

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathField(index: 0, field: \"promethium_pool\")))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form draws the field key only while the effect is LeaveField",
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
