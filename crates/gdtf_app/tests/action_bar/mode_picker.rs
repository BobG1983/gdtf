//! GTW-265 mode 3-toggle sub-panel: offered modes, active mark, mutate-in-place.

use bevy::{prelude::*, ui::Display};
use gdtf_app::test_support::{ModeBurstButton, ModeFullButton, ModeSingleButton};
use gdtf_battle_sim::{Direction, FireMode, ModeKind, StanceKind};
use gdtf_test_utils::press_ui_button;

use super::{harness::*, probes::*};

// =================================================================================
// GTW-265 — the Mode 3-toggle sub-panel (replaces the popup picker).
// =================================================================================

/// GTW-265 / GTW-277 / GTW-284 — the three FIXED mode SEGMENTS always exist; a Single+Burst
/// weapon SHOWS the Single + Burst segments (`Display::Flex`) and HIDES Full
/// (`Display::None`, the mode it does not offer). The active mark sits on the live mode
/// (`single()` default on selection) — the control's `ActiveSegment`. (Adapted to the
/// GTW-277 widget seam: "visible" = `Display::Flex`, "active" = the control's active
/// segment; the only-offered-visible + active-mark CONTRACTS are unchanged.)
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
    // Settle the selection default + the rebuild (.after ApplyTheme) + the active sync.
    app.update();
    app.update();

    // GTW-277/284: the three fixed segments always exist (one each); only Display changes.
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

    // The default-on-select mode is single(), so the Single segment is the active one.
    assert!(
        segment_is_active::<ModeSingleButton>(&mut app),
        "the active mode segment must be the live SelectedFireMode (single() default)",
    );
    assert!(
        !segment_is_active::<ModeBurstButton>(&mut app),
        "the non-active mode segment must NOT be marked active",
    );
}

/// GTW-265 / GTW-277 — selecting the Burst SEGMENT sets `SelectedFireMode` to that weapon's
/// burst spec (read-back identity, never fabricated) and the active mark moves Single ->
/// Burst. Driving the widget = pressing the segment (its `Interaction` → `Pressed`), which
/// `gdtf_ui`'s `select_segment_on_press` turns into a `SegmentSelected` message the
/// `mode_segment_write` listener reads — the SAME downstream contract as the old toggle press.
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
    // Settle default-on-select + the segment rebuild + the active sync.
    app.update();
    app.update();

    // Pre-condition: Single is the active mode by default.
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
        "selecting the Burst segment sets SelectedFireMode to the weapon's burst spec \
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

/// GTW-284 AC1 / GTW-277 — on a weapon/selection change the mode SEGMENTS are MUTATED in
/// place, never despawned/respawned: the segment `Entity` ids stay STABLE across the change
/// and only their `Display` flips to match the new weapon's offered modes (3-mode weapon →
/// all `Display::Flex`; re-select a Single-only weapon → Single Flex, Burst + Full
/// `Display::None`). This is the GTW-284 stable-id invariant preserved through the migration
/// to a `gdtf_ui` `SegmentedControl` — the control is spawned ONCE with all 3 segments and
/// per-segment visibility is toggled via `set_segment_visible`, never a respawn.
///
/// Pin-discriminating: a despawn/respawn body would change the segment ids on the
/// re-selection (failing the id-stability asserts) and would leave the Burst/Full segments
/// absent rather than `Display::None`.
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

    // Capture the three fixed segment ids under weapon A (all three modes offered → Flex).
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

    // Re-select a 1-mode (Single-only) weapon B: the segments MUTATE (Display flips), the
    // entity ids do NOT change (no despawn/respawn).
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

    // Display now matches weapon B's single offered mode.
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
