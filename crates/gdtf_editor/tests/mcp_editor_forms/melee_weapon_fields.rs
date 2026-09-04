use gdtf_battle_sim::weapon::{DamageType, Handedness};
use gdtf_editor::EditorMode;

use crate::{
    rows::{FieldRow, MeleeWeaponFieldRow},
    setup::{form_tab_app_and_client, melee_weapon_draft, set_field},
    support::TestResult,
    values::{DamageTypeRow, HandednessRow},
};

#[test]
fn every_melee_weapon_field_writes_the_draft_and_reads_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;

    let named = set_field(
        &mut app,
        &mut client,
        "(field: MeleeWeapon(Name(\"cleaver\")))",
    )?;
    assert_eq!(
        named.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Name("cleaver".to_owned())),
    );
    assert_eq!(melee_weapon_draft(&app)?.name(), "cleaver");

    let damage = set_field(&mut app, &mut client, "(field: MeleeWeapon(Damage(7)))")?;
    assert_eq!(
        damage.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Damage(7)),
    );
    assert_eq!(*melee_weapon_draft(&app)?.spec().damage, 7);

    let punch = set_field(&mut app, &mut client, "(field: MeleeWeapon(Punch(2)))")?;
    assert_eq!(
        punch.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Punch(2)),
    );
    assert_eq!(*melee_weapon_draft(&app)?.spec().punch, 2);

    let shred = set_field(&mut app, &mut client, "(field: MeleeWeapon(Shred(3)))")?;
    assert_eq!(
        shred.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Shred(3)),
    );
    assert_eq!(*melee_weapon_draft(&app)?.spec().shred, 3);

    let channel = set_field(
        &mut app,
        &mut client,
        "(field: MeleeWeapon(DamageType(Rend)))",
    )?;
    assert_eq!(
        channel.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::DamageType(DamageTypeRow::Rend)),
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().damage_type,
        DamageType::Rend
    );

    let bias = set_field(
        &mut app,
        &mut client,
        "(field: MeleeWeapon(FatalBias(0.25)))",
    )?;
    assert_eq!(
        bias.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::FatalBias(0.25)),
    );
    assert!((*melee_weapon_draft(&app)?.spec().fatal_bias - 0.25).abs() < f32::EPSILON);

    let hands = set_field(
        &mut app,
        &mut client,
        "(field: MeleeWeapon(Handedness(TwoHanded)))",
    )?;
    assert_eq!(
        hands.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Handedness(HandednessRow::TwoHanded)),
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().handedness,
        Handedness::TwoHanded
    );

    let reach = set_field(&mut app, &mut client, "(field: MeleeWeapon(Reach(4)))")?;
    assert_eq!(
        reach.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Reach(4)),
    );
    assert_eq!(*melee_weapon_draft(&app)?.spec().reach, 4);

    let shove = set_field(&mut app, &mut client, "(field: MeleeWeapon(Shove(true)))")?;
    assert_eq!(
        shove.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Shove(true)),
    );
    assert!(*melee_weapon_draft(&app)?.spec().shove);
    Ok(())
}

#[test]
fn a_reach_of_zero_is_stored_as_one_and_the_reply_says_so() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    set_field(&mut app, &mut client, "(field: MeleeWeapon(Reach(5)))")?;
    assert_eq!(
        *melee_weapon_draft(&app)?.spec().reach,
        5,
        "the case starts from a reach that is not the clamp, or storing nothing would pass",
    );

    let written = set_field(&mut app, &mut client, "(field: MeleeWeapon(Reach(0)))")?;
    assert_eq!(
        written.field,
        FieldRow::MeleeWeapon(MeleeWeaponFieldRow::Reach(1)),
        "the form's own handling group writes Reach::new(reach.max(1)), so the reply reads the \
         stored one back rather than the zero that was asked for",
    );
    assert_eq!(*melee_weapon_draft(&app)?.spec().reach, 1);
    Ok(())
}
