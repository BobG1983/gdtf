//! The Melee Weapon draft's fight modes and slot declarations.

use gdtf_battle_sim::equipment::{
    attachments::{AttachmentSlot, SlotCapacity, WeaponSlots},
    weapon::{FightModeKind, FightModeSpec, Strikes, TuCost},
};
use gdtf_content_editor::{EditorMode, structural_swing_mode};

use crate::{
    outcome::unavailable_code,
    refusal::{bad_arguments_detail, refusal_note},
    rows::{ListMemberRow, ListRow},
    setup::{form_tab_app_and_client, list_op, melee_weapon_draft, try_list_op},
    support::TestResult,
    values::{FightModeKindRow, FightModeRow, SlotDeclRow, SlotRow},
};

#[test]
fn adding_a_fight_mode_seeds_the_forms_own_swing() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    let before = melee_weapon_draft(&app)?.fight_modes().len();

    let added = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Add)",
    )?;
    assert_eq!(added.list, ListRow::MeleeWeaponFightModes);
    assert_eq!(added.members.len(), before + 1);
    assert_eq!(
        melee_weapon_draft(&app)?.fight_modes().last().copied(),
        Some(structural_swing_mode()),
        "Add appends the form's own structural swing, so a handler seeding another mode fails \
         here",
    );
    Ok(())
}

#[test]
fn removing_the_last_fight_mode_is_refused_and_leaves_it_in_place() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    let before = melee_weapon_draft(&app)?.fight_modes().to_vec();
    assert_eq!(
        before.len(),
        1,
        "a fresh Melee Weapon draft holds exactly one fight mode, the minimum this case tests",
    );

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Remove(0))",
    )?;
    assert_eq!(unavailable_code(&reply)?, "WrongState");
    assert!(
        refusal_note(&reply)?.contains("at least one"),
        "the note names the minimum the list keeps",
    );
    assert_eq!(
        melee_weapon_draft(&app)?.fight_modes().to_vec(),
        before,
        "the refused op left the one mode in place",
    );
    Ok(())
}

#[test]
fn a_fight_mode_index_past_the_end_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Add)",
    )?;
    let before = melee_weapon_draft(&app)?.fight_modes().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the list must hold more than the minimum, or the min-one refusal would mask the \
         out-of-bounds answer this case is about",
    );

    let removed = try_list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Remove(7))",
    )?;
    bad_arguments_detail(&removed)?;
    assert_eq!(
        melee_weapon_draft(&app)?.fight_modes().to_vec(),
        before,
        "the refused remove left the fight modes exactly as they were",
    );

    let edited = try_list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: SetAt(7, FightMode((kind: Thrust, tu_cost: 7, \
         strikes: 3))))",
    )?;
    bad_arguments_detail(&edited)?;
    assert_eq!(
        melee_weapon_draft(&app)?.fight_modes().to_vec(),
        before,
        "the refused edit wrote no mode, so an index past the end answers rather than doing \
         nothing quietly",
    );
    Ok(())
}

#[test]
fn a_slot_add_then_edit_writes_the_declaration_the_reply_reads_back() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    assert!(
        melee_weapon_draft(&app)?
            .spec()
            .slots
            .declarations()
            .is_empty(),
        "a fresh draft declares no slot, or the add below would prove nothing",
    );

    let added = list_op(&mut app, &mut client, "(list: MeleeWeaponSlots, op: Add)")?;
    assert_eq!(
        added.members,
        vec![ListMemberRow::Slot(SlotDeclRow {
            slot:     SlotRow::Muzzle,
            capacity: 1,
        })],
        "Add seeds the sim's own default declaration",
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().slots.declarations(),
        [WeaponSlots::DEFAULT_DECLARATION],
    );

    let edited = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponSlots, op: SetAt(0, Slot((slot: Pommel, capacity: 3))))",
    )?;
    assert_eq!(
        edited.members,
        vec![ListMemberRow::Slot(SlotDeclRow {
            slot:     SlotRow::Pommel,
            capacity: 3,
        })],
        "the reply reads the rewritten declaration back off the draft",
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().slots.declarations(),
        [(AttachmentSlot::Pommel, SlotCapacity::new(3))],
    );
    Ok(())
}

#[test]
fn a_fight_mode_edit_at_an_index_replaces_only_that_mode() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: Add)",
    )?;
    let before = melee_weapon_draft(&app)?.fight_modes().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the case edits the second mode, so two must be in place first",
    );

    let edited = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponFightModes, op: SetAt(1, FightMode((kind: Thrust, tu_cost: 7, \
         strikes: 3))))",
    )?;
    let written = FightModeSpec::new(FightModeKind::Thrust, TuCost::new(7), Strikes::new(3));
    assert_eq!(
        edited.members,
        vec![
            ListMemberRow::FightMode(FightModeRow {
                kind:    FightModeKindRow::Swing,
                tu_cost: *before[0].tu_cost,
                strikes: *before[0].strikes,
            }),
            ListMemberRow::FightMode(FightModeRow {
                kind:    FightModeKindRow::Thrust,
                tu_cost: 7,
                strikes: 3,
            }),
        ],
        "the reply reads both modes back off the draft, the edited one rewritten",
    );
    let after = melee_weapon_draft(&app)?.fight_modes().to_vec();
    assert_eq!(
        after,
        vec![before[0], written],
        "the edit rewrites index 1 and leaves index 0 as it was, so a handler writing index 0 or \
         writing nothing fails here",
    );
    Ok(())
}

#[test]
fn a_slot_remove_takes_the_declaration_off_with_no_minimum() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    list_op(&mut app, &mut client, "(list: MeleeWeaponSlots, op: Add)")?;
    list_op(&mut app, &mut client, "(list: MeleeWeaponSlots, op: Add)")?;
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponSlots, op: SetAt(1, Slot((slot: Pommel, capacity: 2))))",
    )?;

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponSlots, op: Remove(0))",
    )?;
    assert_eq!(
        removed.members,
        vec![ListMemberRow::Slot(SlotDeclRow {
            slot:     SlotRow::Pommel,
            capacity: 2,
        })],
        "the remove takes the first declaration off and leaves the second",
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().slots.declarations(),
        [(AttachmentSlot::Pommel, SlotCapacity::new(2))],
    );

    let emptied = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponSlots, op: Remove(0))",
    )?;
    assert!(
        emptied.members.is_empty(),
        "the slot list keeps no minimum, so the last declaration comes off too",
    );
    assert!(
        melee_weapon_draft(&app)?
            .spec()
            .slots
            .declarations()
            .is_empty(),
    );
    Ok(())
}
