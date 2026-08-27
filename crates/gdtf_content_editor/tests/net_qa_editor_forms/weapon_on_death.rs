//! The Weapon form's on-death tick box, its variant combo and each variant's own payload.

use bevy::app::App;
use gdtf_battle_sim::{
    effects::on_death::{ExplodeDamage, OnDeathEffect},
    weapon::{BlastRadius, DamageType, HitType},
};
use gdtf_content_editor::EditorMode;

use crate::{
    outcome::unavailable_code,
    refusal::refusal_note,
    rows::{FieldRow, WeaponFieldRow},
    setup::{form_tab_app_and_client, set_field, try_set_field, weapon_draft},
    support::{TestError, TestResult},
    values::{DamageTypeRow, HitTypeRow, OnDeathVariantRow},
};

// The effect the draft authors right now, or why it authors none.
fn effect(app: &App) -> Result<OnDeathEffect, TestError> {
    match weapon_draft(app)?.spec().on_death.clone() {
        Some(held) => Ok(held),
        None => Err("the Weapon draft authors no on-death effect".into()),
    }
}

#[test]
fn the_tick_box_seeds_the_forms_own_explode_template_and_its_rows_then_write_it() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert!(
        weapon_draft(&app)?.spec().on_death.is_none(),
        "a fresh Weapon draft authors no on-death effect, or the tick box would prove nothing",
    );

    let on = set_field(&mut app, &mut client, "(field: Weapon(OnDeath(true)))")?;
    assert_eq!(on.field, FieldRow::Weapon(WeaponFieldRow::OnDeath(true)));
    assert_eq!(
        effect(&app)?,
        OnDeathEffect::Explode {
            hit_type:    HitType::Single,
            damage:      ExplodeDamage::new(0),
            damage_type: DamageType::Kinetic,
        },
        "the tick box seeds the same blank explode the form's own checkbox seeds",
    );

    let geometry = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathHitType(Blast(radius: 2))))",
    )?;
    assert_eq!(
        geometry.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathHitType(HitTypeRow::Blast {
            radius: 2,
        })),
    );
    let damage = set_field(&mut app, &mut client, "(field: Weapon(OnDeathDamage(12)))")?;
    assert_eq!(
        damage.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathDamage(12)),
    );
    let channel = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathDamageType(Blast)))",
    )?;
    assert_eq!(
        channel.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathDamageType(DamageTypeRow::Blast)),
    );

    let OnDeathEffect::Explode {
        hit_type,
        damage,
        damage_type,
    } = effect(&app)?
    else {
        return Err("the three payload writes left the draft on the other variant".into());
    };
    assert_eq!(
        hit_type,
        HitType::Blast {
            radius: BlastRadius::new(2),
        }
    );
    assert_eq!(*damage, 12);
    assert_eq!(damage_type, DamageType::Blast);

    let off = set_field(&mut app, &mut client, "(field: Weapon(OnDeath(false)))")?;
    assert_eq!(off.field, FieldRow::Weapon(WeaponFieldRow::OnDeath(false)));
    assert!(weapon_draft(&app)?.spec().on_death.is_none());
    Ok(())
}

#[test]
fn the_variant_pick_swaps_the_template_and_the_field_key_then_writes() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    set_field(&mut app, &mut client, "(field: Weapon(OnDeath(true)))")?;

    let picked = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(LeaveField)))",
    )?;
    assert_eq!(
        picked.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathVariant(
            OnDeathVariantRow::LeaveField
        )),
    );

    let keyed = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathField(\"promethium_pool\")))",
    )?;
    assert_eq!(
        keyed.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathField("promethium_pool".to_owned())),
    );
    let OnDeathEffect::LeaveField { field } = effect(&app)? else {
        return Err("the variant pick left the draft on Explode".into());
    };
    assert_eq!(field.as_str(), "promethium_pool");

    let back = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(Explode)))",
    )?;
    assert_eq!(
        back.field,
        FieldRow::Weapon(WeaponFieldRow::OnDeathVariant(OnDeathVariantRow::Explode)),
    );
    assert!(
        matches!(effect(&app)?, OnDeathEffect::Explode { .. }),
        "picking the other variant swaps in that variant's blank template",
    );
    Ok(())
}

#[test]
fn an_on_death_payload_write_with_the_tick_box_off_is_refused_and_seeds_no_effect() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;

    for arguments in [
        "(field: Weapon(OnDeathVariant(LeaveField)))",
        "(field: Weapon(OnDeathHitType(Single)))",
        "(field: Weapon(OnDeathDamage(12)))",
        "(field: Weapon(OnDeathDamageType(Blast)))",
        "(field: Weapon(OnDeathField(\"promethium_pool\")))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "the form draws `{arguments}` only while the on death tick box is on",
        );
        assert!(
            refusal_note(&reply)?.contains("on death effect tick box"),
            "the note names the toggle that is off",
        );
        assert!(
            weapon_draft(&app)?.spec().on_death.is_none(),
            "the refused write must not seed an effect as a side effect",
        );
    }
    Ok(())
}

#[test]
fn an_explode_payload_write_while_the_effect_leaves_a_field_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    set_field(&mut app, &mut client, "(field: Weapon(OnDeath(true)))")?;
    set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathVariant(LeaveField)))",
    )?;
    let before = effect(&app)?;

    for arguments in [
        "(field: Weapon(OnDeathHitType(Single)))",
        "(field: Weapon(OnDeathDamage(12)))",
        "(field: Weapon(OnDeathDamageType(Blast)))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "the form draws `{arguments}` only while the effect is Explode",
        );
        assert!(
            refusal_note(&reply)?.contains("LeaveField"),
            "the note names the variant the draft is on",
        );
        assert_eq!(
            effect(&app)?,
            before,
            "the refused write left the effect exactly as it was",
        );
    }
    Ok(())
}

#[test]
fn a_field_key_write_while_the_effect_explodes_is_refused() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    set_field(&mut app, &mut client, "(field: Weapon(OnDeath(true)))")?;
    let before = effect(&app)?;

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Weapon(OnDeathField(\"promethium_pool\")))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form draws the field key only while the effect is LeaveField",
    );
    assert!(
        refusal_note(&reply)?.contains("Explode"),
        "the note names the variant the draft is on",
    );
    assert_eq!(
        effect(&app)?,
        before,
        "the refused write left the effect exactly as it was",
    );
    Ok(())
}
