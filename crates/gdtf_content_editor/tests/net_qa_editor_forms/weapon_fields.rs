//! The Weapon form's single-value rows, written the way its own def panel writes them.

use gdtf_battle_sim::weapon::{DamageType, Handedness, TrajectoryStyle};
use gdtf_content_editor::EditorMode;

use crate::{
    outcome::unavailable_code,
    refusal::refusal_note,
    rows::{FieldRow, WeaponFieldRow},
    setup::{form_tab_app_and_client, set_field, try_set_field, weapon_draft},
    support::TestResult,
    values::{DamageTypeRow, HandednessRow, TrajectoryRow},
};

#[test]
fn the_weapon_name_and_every_stat_row_write_the_draft_and_read_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;

    let named = set_field(&mut app, &mut client, "(field: Weapon(Name(\"stub gun\")))")?;
    assert_eq!(
        named.field,
        FieldRow::Weapon(WeaponFieldRow::Name("stub gun".to_owned())),
    );
    assert_eq!(weapon_draft(&app)?.name(), "stub gun");

    let spread = set_field(&mut app, &mut client, "(field: Weapon(BaseSpread(0.125)))")?;
    assert_eq!(
        spread.field,
        FieldRow::Weapon(WeaponFieldRow::BaseSpread(0.125)),
    );
    assert!((*weapon_draft(&app)?.spec().base_spread - 0.125).abs() < f32::EPSILON);

    let accuracy = set_field(&mut app, &mut client, "(field: Weapon(Accuracy(0.75)))")?;
    assert_eq!(
        accuracy.field,
        FieldRow::Weapon(WeaponFieldRow::Accuracy(0.75)),
    );
    assert!((*weapon_draft(&app)?.spec().accuracy - 0.75).abs() < f32::EPSILON);

    let kickback = set_field(&mut app, &mut client, "(field: Weapon(Kickback(0.5)))")?;
    assert_eq!(
        kickback.field,
        FieldRow::Weapon(WeaponFieldRow::Kickback(0.5)),
    );
    assert!((*weapon_draft(&app)?.spec().kickback - 0.5).abs() < f32::EPSILON);
    Ok(())
}

#[test]
fn every_damage_group_row_writes_the_draft_and_reads_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;

    let damage = set_field(&mut app, &mut client, "(field: Weapon(Damage(9)))")?;
    assert_eq!(damage.field, FieldRow::Weapon(WeaponFieldRow::Damage(9)));
    assert_eq!(*weapon_draft(&app)?.spec().damage, 9);

    let punch = set_field(&mut app, &mut client, "(field: Weapon(Punch(3)))")?;
    assert_eq!(punch.field, FieldRow::Weapon(WeaponFieldRow::Punch(3)));
    assert_eq!(*weapon_draft(&app)?.spec().punch, 3);

    let shred = set_field(&mut app, &mut client, "(field: Weapon(Shred(2)))")?;
    assert_eq!(shred.field, FieldRow::Weapon(WeaponFieldRow::Shred(2)));
    assert_eq!(*weapon_draft(&app)?.spec().shred, 2);

    let channel = set_field(&mut app, &mut client, "(field: Weapon(DamageType(Plasma)))")?;
    assert_eq!(
        channel.field,
        FieldRow::Weapon(WeaponFieldRow::DamageType(DamageTypeRow::Plasma)),
    );
    assert_eq!(weapon_draft(&app)?.spec().damage_type, DamageType::Plasma);

    let bias = set_field(&mut app, &mut client, "(field: Weapon(FatalBias(0.25)))")?;
    assert_eq!(
        bias.field,
        FieldRow::Weapon(WeaponFieldRow::FatalBias(0.25)),
    );
    assert!((*weapon_draft(&app)?.spec().fatal_bias - 0.25).abs() < f32::EPSILON);

    let hands = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(Handedness(TwoHanded)))",
    )?;
    assert_eq!(
        hands.field,
        FieldRow::Weapon(WeaponFieldRow::Handedness(HandednessRow::TwoHanded)),
    );
    assert_eq!(weapon_draft(&app)?.spec().handedness, Handedness::TwoHanded);
    Ok(())
}

#[test]
fn every_handling_and_magazine_row_writes_the_draft_and_reads_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert_eq!(
        weapon_draft(&app)?.spec().trajectory,
        TrajectoryStyle::Straight,
        "the case asks for the trajectory the draft is not on, or writing nothing would pass",
    );

    let arc = set_field(&mut app, &mut client, "(field: Weapon(Trajectory(Arc)))")?;
    assert_eq!(
        arc.field,
        FieldRow::Weapon(WeaponFieldRow::Trajectory(TrajectoryRow::Arc)),
    );
    assert_eq!(weapon_draft(&app)?.spec().trajectory, TrajectoryStyle::Arc);

    let stable = set_field(&mut app, &mut client, "(field: Weapon(Stable(true)))")?;
    assert_eq!(stable.field, FieldRow::Weapon(WeaponFieldRow::Stable(true)));
    assert!(*weapon_draft(&app)?.spec().stable);

    let shove = set_field(&mut app, &mut client, "(field: Weapon(Shove(true)))")?;
    assert_eq!(shove.field, FieldRow::Weapon(WeaponFieldRow::Shove(true)));
    assert!(*weapon_draft(&app)?.spec().shove);

    let size = set_field(&mut app, &mut client, "(field: Weapon(MagazineSize(24)))")?;
    assert_eq!(
        size.field,
        FieldRow::Weapon(WeaponFieldRow::MagazineSize(24)),
    );
    assert_eq!(*weapon_draft(&app)?.spec().magazine.size(), 24);

    let reload = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(MagazineReloadTu(6)))",
    )?;
    assert_eq!(
        reload.field,
        FieldRow::Weapon(WeaponFieldRow::MagazineReloadTu(6)),
    );
    assert_eq!(*weapon_draft(&app)?.spec().magazine.reload_tu(), 6);
    Ok(())
}

#[test]
fn the_dot_tick_box_seeds_a_profile_and_its_three_rows_then_write_it() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert!(
        weapon_draft(&app)?.spec().dot.is_none(),
        "a fresh Weapon draft authors no DOT, or the tick box below would prove nothing",
    );

    let on = set_field(&mut app, &mut client, "(field: Weapon(Dot(true)))")?;
    assert_eq!(on.field, FieldRow::Weapon(WeaponFieldRow::Dot(true)));
    assert!(weapon_draft(&app)?.spec().dot.is_some());

    let damage = set_field(&mut app, &mut client, "(field: Weapon(DotDamage(4)))")?;
    assert_eq!(damage.field, FieldRow::Weapon(WeaponFieldRow::DotDamage(4)),);
    let turns = set_field(&mut app, &mut client, "(field: Weapon(DotTurns(3)))")?;
    assert_eq!(turns.field, FieldRow::Weapon(WeaponFieldRow::DotTurns(3)));
    let channel = set_field(
        &mut app,
        &mut client,
        "(field: Weapon(DotDamageType(Chem)))",
    )?;
    assert_eq!(
        channel.field,
        FieldRow::Weapon(WeaponFieldRow::DotDamageType(DamageTypeRow::Chem)),
    );

    let Some(profile) = weapon_draft(&app)?.spec().dot else {
        return Err("the three writes above landed on a profile the draft no longer holds".into());
    };
    assert_eq!(*profile.damage, 4);
    assert_eq!(profile.turns.get(), 3);
    assert_eq!(profile.damage_type, DamageType::Chem);

    let off = set_field(&mut app, &mut client, "(field: Weapon(Dot(false)))")?;
    assert_eq!(off.field, FieldRow::Weapon(WeaponFieldRow::Dot(false)));
    assert!(weapon_draft(&app)?.spec().dot.is_none());
    Ok(())
}

#[test]
fn a_dot_payload_write_with_the_tick_box_off_is_refused_and_seeds_no_profile() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert!(
        weapon_draft(&app)?.spec().dot.is_none(),
        "the case only proves the gate while the tick box is off",
    );

    for arguments in [
        "(field: Weapon(DotDamage(4)))",
        "(field: Weapon(DotTurns(3)))",
        "(field: Weapon(DotDamageType(Chem)))",
    ] {
        let reply = try_set_field(&mut app, &mut client, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "the form draws `{arguments}` only while the dot tick box is on",
        );
        assert!(
            refusal_note(&reply)?.contains("dot tick box"),
            "the note names the toggle that is off",
        );
        assert!(
            weapon_draft(&app)?.spec().dot.is_none(),
            "the refused write must not seed a profile as a side effect",
        );
    }
    Ok(())
}

#[test]
fn a_dot_turns_of_zero_is_stored_as_one_and_the_reply_says_so() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    set_field(&mut app, &mut client, "(field: Weapon(Dot(true)))")?;
    set_field(&mut app, &mut client, "(field: Weapon(DotTurns(5)))")?;
    assert_eq!(
        weapon_draft(&app)?.spec().dot.map(|dot| dot.turns.get()),
        Some(5),
        "the case starts from a count that is not the clamp, or storing nothing would pass",
    );

    let written = set_field(&mut app, &mut client, "(field: Weapon(DotTurns(0)))")?;
    assert_eq!(
        written.field,
        FieldRow::Weapon(WeaponFieldRow::DotTurns(1)),
        "the form's own drag commits through weapon_form::dot_turns_from_raw, which turns 0 into \
         1, so the reply reads the stored count back rather than the zero that was asked for",
    );
    assert_eq!(
        weapon_draft(&app)?.spec().dot.map(|dot| dot.turns.get()),
        Some(1),
    );
    Ok(())
}
