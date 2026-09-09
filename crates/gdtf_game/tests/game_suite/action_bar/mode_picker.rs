use bevy::{prelude::*, ui::Display};
use cobalt_test_utils::press_ui_button;
use gdtf_battle_sim::{
    prelude::{Direction, StanceKind},
    weapon::{FireMode, ModeKind},
};
use gdtf_game::test_support::{ModeBurstButton, ModeFullButton, ModeSingleButton};

use super::{harness::*, probes::*};

#[test]
fn mode_panel_spawns_only_offered_modes_and_marks_active() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    assert_eq!(
        count_with::<ModeSingleButton>(&mut app),
        1,
        "the fixed Single mode segment exists exactly once",
    );
    assert_eq!(
        count_with::<ModeBurstButton>(&mut app),
        1,
        "the fixed Burst mode segment exists exactly once",
    );
    assert_eq!(
        count_with::<ModeFullButton>(&mut app),
        1,
        "the fixed Full mode segment exists exactly once (it is collapsed, not despawned)",
    );
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "a Single+Burst weapon shows the Single mode segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::Flex),
        "a Single+Burst weapon shows the Burst mode segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::None),
        "a Single+Burst weapon HIDES the Full mode segment (Display::None — not offered)",
    );

    assert!(
        segment_is_active::<ModeSingleButton>(&mut app),
        "the active mode segment must be the mode the gun is on, which with none picked is its \
         own single()",
    );
    assert!(
        !segment_is_active::<ModeBurstButton>(&mut app),
        "the non-active mode segment must NOT be marked active",
    );
}

#[test]
fn clicking_burst_toggle_sets_mode_and_moves_active_mark() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);
    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(single),
        "the default-on-select mode must be single()",
    );

    let Some(burst_segment) = require_button::<ModeBurstButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, burst_segment);
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(burst),
        "selecting the Burst segment puts the weapon's own burst spec on the gun \
         (read-back, not fabricated)",
    );
    assert!(
        segment_is_active::<ModeBurstButton>(&mut app),
        "the active mark must move to the Burst segment",
    );
    assert!(
        !segment_is_active::<ModeSingleButton>(&mut app),
        "the Single segment must no longer be active after Burst is chosen",
    );
}

#[test]
fn mode_toggles_mutate_in_place_keeping_stable_ids() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);

    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    let Some(single_a) = single_with::<ModeSingleButton>(&mut app) else {
        return;
    };
    let Some(burst_a) = single_with::<ModeBurstButton>(&mut app) else {
        return;
    };
    let Some(full_a) = single_with::<ModeFullButton>(&mut app) else {
        return;
    };
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "weapon A (Single+Burst+Full) shows the Single segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::Flex),
        "weapon A shows the Burst segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::Flex),
        "weapon A shows the Full segment (Display::Flex)",
    );

    arm_and_select(
        &mut app,
        FireMode::new(vec![single]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    assert_eq!(
        single_with::<ModeSingleButton>(&mut app),
        Some(single_a),
        "the Single mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );
    assert_eq!(
        single_with::<ModeBurstButton>(&mut app),
        Some(burst_a),
        "the Burst mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );
    assert_eq!(
        single_with::<ModeFullButton>(&mut app),
        Some(full_a),
        "the Full mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );

    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "weapon B (Single-only) keeps the Single segment shown (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::None),
        "weapon B (Single-only) HIDES the Burst segment (Display::None — not offered), not despawns it",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::None),
        "weapon B (Single-only) HIDES the Full segment (Display::None — not offered), not despawns it",
    );
}
