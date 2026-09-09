//! The Weapon draft's fire modes and slot declarations.

use gdtf_battle_sim::{
    equipment::attachments::{AttachmentSlot, SlotCapacity, WeaponSlots},
    weapon::{FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};
use gdtf_editor::EditorMode;

use crate::{
    mcp_editor_forms::{
        refusal::refusal_note,
        rows::{ListMemberRow, ListRow},
        setup::{form_tab_app_and_client, list_op, try_list_op, weapon_draft},
        values::{FireModeRow, HitTypeRow, ModeKindRow, SlotDeclRow, SlotRow},
    },
    mcp_shared::{
        bad_arguments::bad_arguments_detail, outcome::unavailable_code, support::TestResult,
    },
};

#[test]
fn adding_a_fire_mode_appends_one_and_a_remove_takes_it_back_off() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    let before = weapon_draft(&app)?.fire_modes().to_vec();

    let added = list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    assert_eq!(added.list, ListRow::WeaponFireModes);
    assert_eq!(added.members.len(), before.len() + 1);
    assert_eq!(
        weapon_draft(&app)?.fire_modes().first().copied(),
        before.first().copied(),
        "Add appends through the draft's own setter and leaves the mode already authored",
    );

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: Remove(1))",
    )?;
    assert_eq!(removed.members.len(), before.len());
    Ok(())
}

#[test]
fn removing_the_last_fire_mode_is_refused_and_leaves_it_in_place() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: Remove(1))",
    )?;
    let before = weapon_draft(&app)?.fire_modes().to_vec();
    assert_eq!(
        before.len(),
        1,
        "the case removes down to the minimum first, so the refusal below is the min-one rule",
    );

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: Remove(0))",
    )?;
    assert_eq!(unavailable_code(&reply)?, "WrongState");
    assert!(
        refusal_note(&reply)?.contains("at least one"),
        "the note names the minimum the list keeps",
    );
    assert_eq!(
        weapon_draft(&app)?.fire_modes().to_vec(),
        before,
        "the refused op left the one mode in place, so a handler calling Vec::remove fails here",
    );
    Ok(())
}

#[test]
fn a_fire_mode_edit_at_an_index_rewrites_all_five_of_its_fields() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    let before = weapon_draft(&app)?.fire_modes().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the case edits the second mode, so two must be in place first",
    );

    let edited = list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: SetAt(1, FireMode((kind: Burst, cone_mult: 1.5, \
         tu_percent: 0.4, shots: 3, hit_type: Line(range: 4)))))",
    )?;
    assert_eq!(
        edited.members.get(1),
        Some(&ListMemberRow::FireMode(FireModeRow {
            kind:       ModeKindRow::Burst,
            cone_mult:  1.5,
            tu_percent: 0.4,
            shots:      3,
            hit_type:   HitTypeRow::Line { range: 4 },
        })),
        "the reply reads the rewritten mode back off the draft, hit geometry included",
    );
    let written = FireModeSpec::with_hit_type(
        ModeKind::Burst,
        ModeConeMult::new(1.5),
        ModeTuPercent::new(0.4),
        ModeShots::new(3),
        HitType::Line {
            range: gdtf_battle_sim::weapon::AoeRange::new(4),
        },
    );
    assert_eq!(
        weapon_draft(&app)?.fire_modes().to_vec(),
        vec![before[0], written],
        "the edit rewrites index 1 and leaves index 0 as it was",
    );
    Ok(())
}

#[test]
fn a_slot_add_then_edit_then_remove_writes_the_declaration_the_reply_reads_back() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    assert!(
        weapon_draft(&app)?.spec().slots.declarations().is_empty(),
        "a fresh draft declares no slot, or the add below would prove nothing",
    );

    let added = list_op(&mut app, &mut client, "(list: WeaponSlots, op: Add)")?;
    assert_eq!(
        added.members,
        vec![ListMemberRow::Slot(SlotDeclRow {
            slot:     SlotRow::Muzzle,
            capacity: 1,
        })],
        "Add seeds the sim's own default declaration",
    );
    assert_eq!(
        weapon_draft(&app)?.spec().slots.declarations(),
        [WeaponSlots::DEFAULT_DECLARATION],
    );

    let edited = list_op(
        &mut app,
        &mut client,
        "(list: WeaponSlots, op: SetAt(0, Slot((slot: Sight, capacity: 2))))",
    )?;
    assert_eq!(
        edited.members,
        vec![ListMemberRow::Slot(SlotDeclRow {
            slot:     SlotRow::Sight,
            capacity: 2,
        })],
    );
    assert_eq!(
        weapon_draft(&app)?.spec().slots.declarations(),
        [(AttachmentSlot::Sight, SlotCapacity::new(2))],
    );

    let removed = list_op(&mut app, &mut client, "(list: WeaponSlots, op: Remove(0))")?;
    assert!(
        removed.members.is_empty(),
        "the slot list keeps no minimum, so the last declaration comes off too",
    );
    Ok(())
}

#[test]
fn an_index_past_the_end_of_a_weapon_list_is_bad_arguments_and_writes_nothing() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    let before = weapon_draft(&app)?.fire_modes().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the list must hold more than the minimum, or the min-one refusal would mask the \
         out-of-bounds answer this case is about",
    );

    let removed = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: Remove(7))",
    )?;
    bad_arguments_detail(&removed)?;
    let edited = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: SetAt(7, FireMode((kind: Single, cone_mult: 1.0, \
         tu_percent: 0.0, shots: 1, hit_type: Single))))",
    )?;
    bad_arguments_detail(&edited)?;
    let slotted = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponSlots, op: SetAt(3, Slot((slot: Rail, capacity: 1))))",
    )?;
    bad_arguments_detail(&slotted)?;
    assert_eq!(
        weapon_draft(&app)?.fire_modes().to_vec(),
        before,
        "the refused operations wrote nothing",
    );
    assert!(weapon_draft(&app)?.spec().slots.declarations().is_empty());
    Ok(())
}

#[test]
fn a_member_a_weapon_list_does_not_draw_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(&mut app, &mut client, "(list: WeaponSlots, op: Add)")?;
    let before = weapon_draft(&app)?.spec().slots.declarations().to_vec();

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponSlots, op: SetAt(0, FireMode((kind: Single, cone_mult: 1.0, tu_percent: \
         0.0, shots: 1, hit_type: Single))))",
    )?;
    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("WeaponSlots"),
        "the detail names the list the member does not belong to, got `{detail}`",
    );
    assert_eq!(
        weapon_draft(&app)?.spec().slots.declarations().to_vec(),
        before,
        "the refused edit left the declaration exactly as it was",
    );
    Ok(())
}

#[test]
fn a_reorder_or_a_toggle_against_the_weapon_fire_modes_slots_or_attachments_is_bad_arguments()
-> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    let before = weapon_draft(&app)?.fire_modes().to_vec();

    for arguments in [
        "(list: WeaponFireModes, op: MoveUp(1))",
        "(list: WeaponFireModes, op: MoveDown(0))",
        "(list: WeaponSlots, op: MoveUp(0))",
        "(list: WeaponAttachments, op: MoveDown(0))",
    ] {
        let reply = try_list_op(&mut app, &mut client, arguments)?;
        let detail = bad_arguments_detail(&reply)?;
        assert!(
            detail.contains("reorder"),
            "the fire modes, the slots and the fitted attachments draw no reorder buttons, so \
             `{arguments}` is refused by the handler and the detail says why, got `{detail}`",
        );
    }

    let toggled = try_list_op(
        &mut app,
        &mut client,
        "(list: WeaponFireModes, op: Toggle(EntrySide(East)))",
    )?;
    let detail = bad_arguments_detail(&toggled)?;
    assert!(
        detail.contains("toggle"),
        "the fire-mode list is authored by add, remove and rewrite, got `{detail}`",
    );
    assert_eq!(
        weapon_draft(&app)?.fire_modes().to_vec(),
        before,
        "every refused operation left the fire modes exactly as they were",
    );
    Ok(())
}
