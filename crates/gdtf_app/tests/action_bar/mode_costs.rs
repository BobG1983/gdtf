//! GTW-303 per-mode TU-cost sub-lines, incl. the aim premium.

use bevy::prelude::*;
use gdtf_app::test_support::{ModeBurstButton, ModeFullButton, ModeSingleButton};
use gdtf_battle_sim::{
    Aiming, Direction, FireMode, FireModeSpec, ModeKind, StanceKind, TuMax, mode_tu_cost,
    tuning::CombatTuning,
};

use super::{harness::*, probes::*};

// ---------------------------------------------------------------------------------
// GTW-303 — each firemode segment shows a TU-cost SUB-LINE ("{n} TU") = the EXACT value
// `mode_tu_cost` charges, hip-fire when Aim is off and base×premium when Aim is on; a
// non-offered mode / unarmed selection shows NO cost line. The expected value is computed
// with the SAME `mode_tu_cost` the production system reuses, so the display==charge guarantee
// is pinned (no hardcoded magic number that could drift from the tuning).
// ---------------------------------------------------------------------------------

/// GTW-303 — with a known `TuMax` and three distinct `tu_percent` modes, each OFFERED segment's
/// cost sub-line equals `mode_tu_cost(spec, tu_max, Aiming(false), tuning)` formatted "{n} TU"
/// while Aim is OFF (hip-fire). All three modes are covered (different `tu_percent`s → distinct
/// costs), proving the cost is per-mode, not a single shared value.
#[test]
fn mode_cost_lines_show_hip_fire_cost_per_mode() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);
    let tu_max = TuMax::new(100);
    let tuning = CombatTuning::default();
    let aiming_off = Aiming::new(false);

    let mut app = battle_running_app();
    arm_and_select_with_tu(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        tu_max,
        StanceKind::Standing,
        Direction::North,
    );
    // Settle the selection default, the segment rebuild (.after ApplyTheme), and the cost-line
    // write (.after rebuild).
    app.update();
    app.update();

    // Expected = the SAME sim fn the production system reuses (display==charge), per mode.
    let want = |mode: &FireModeSpec| {
        let cost = mode_tu_cost(mode, &tu_max, &aiming_off, &tuning);
        format!("{} TU", *cost)
    };
    assert_eq!(
        segment_sub_line::<ModeSingleButton>(&mut app),
        Some(want(&single)),
        "the Single segment shows its hip-fire TU cost",
    );
    assert_eq!(
        segment_sub_line::<ModeBurstButton>(&mut app),
        Some(want(&burst)),
        "the Burst segment shows its hip-fire TU cost (a different tu_percent → a distinct cost)",
    );
    assert_eq!(
        segment_sub_line::<ModeFullButton>(&mut app),
        Some(want(&full)),
        "the Full segment shows its hip-fire TU cost (a third distinct tu_percent)",
    );
}

/// GTW-303 — the displayed cost INCLUDES the aim premium when Aim is ON, and REVERTS to the
/// hip-fire base when Aim is OFF. With the default tuning premium (×1.5) the aimed cost is
/// strictly greater than the hip-fire cost for every offered mode, and equals
/// `mode_tu_cost(.., Aiming(true), ..)`.
///
/// Pin-discriminating: if the system dropped the aim premium (computed the cost with a fixed
/// `Aiming(false)`), the aimed sub-line would equal the hip-fire one and the `!=` /
/// `aimed > hip` asserts would fail — so a regression that ignores `Aiming` is caught.
#[test]
fn mode_cost_lines_include_aim_premium_and_revert() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);
    let tu_max = TuMax::new(100);
    let tuning = CombatTuning::default();
    let aiming_off = Aiming::new(false);
    let aiming_on = Aiming::new(true);

    let mut app = battle_running_app();
    arm_and_select_with_tu(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        tu_max,
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    let hip =
        |mode: &FireModeSpec| format!("{} TU", *mode_tu_cost(mode, &tu_max, &aiming_off, &tuning));
    let aimed =
        |mode: &FireModeSpec| format!("{} TU", *mode_tu_cost(mode, &tu_max, &aiming_on, &tuning));

    // Pre-condition: hip-fire (Aim off) cost, captured per mode.
    assert_eq!(
        segment_sub_line::<ModeSingleButton>(&mut app),
        Some(hip(&single))
    );
    assert_eq!(
        segment_sub_line::<ModeBurstButton>(&mut app),
        Some(hip(&burst))
    );
    assert_eq!(
        segment_sub_line::<ModeFullButton>(&mut app),
        Some(hip(&full))
    );

    // Flip Aim ON (Changed<Aiming> drives the recompute): every cost line updates to the aimed
    // value, strictly greater than hip-fire (the default ×1.5 premium).
    set_selected_aiming(&mut app, true);
    for (marker_cost_aimed, hip_cost, mode) in [
        (
            segment_sub_line::<ModeSingleButton>(&mut app),
            hip(&single),
            single,
        ),
        (
            segment_sub_line::<ModeBurstButton>(&mut app),
            hip(&burst),
            burst,
        ),
        (
            segment_sub_line::<ModeFullButton>(&mut app),
            hip(&full),
            full,
        ),
    ] {
        assert_eq!(
            marker_cost_aimed,
            Some(aimed(&mode)),
            "with Aim ON, the cost line includes the aim premium (== mode_tu_cost aimed)",
        );
        assert_ne!(
            marker_cost_aimed,
            Some(hip_cost),
            "the aimed cost must DIFFER from the hip-fire cost (the premium is applied)",
        );
    }

    // Flip Aim OFF: every cost line reverts to the hip-fire base.
    set_selected_aiming(&mut app, false);
    assert_eq!(
        segment_sub_line::<ModeSingleButton>(&mut app),
        Some(hip(&single)),
        "with Aim OFF again, the Single cost line reverts to hip-fire",
    );
    assert_eq!(
        segment_sub_line::<ModeBurstButton>(&mut app),
        Some(hip(&burst)),
        "with Aim OFF again, the Burst cost line reverts to hip-fire",
    );
    assert_eq!(
        segment_sub_line::<ModeFullButton>(&mut app),
        Some(hip(&full)),
        "with Aim OFF again, the Full cost line reverts to hip-fire",
    );
}

/// GTW-303 — a NON-OFFERED mode shows NO cost line: a Single+Burst weapon offers no Full mode,
/// so the (collapsed) Full segment has no sub-line node, while the two offered segments do show
/// their cost. This proves the system clears the sub-line of a mode the weapon does not offer.
#[test]
fn non_offered_mode_has_no_cost_line() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let tu_max = TuMax::new(80);
    let tuning = CombatTuning::default();
    let aiming_off = Aiming::new(false);

    let mut app = battle_running_app();
    arm_and_select_with_tu(
        &mut app,
        FireMode::new(vec![single, burst]),
        tu_max,
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    let want =
        |mode: &FireModeSpec| format!("{} TU", *mode_tu_cost(mode, &tu_max, &aiming_off, &tuning));
    assert_eq!(
        segment_sub_line::<ModeSingleButton>(&mut app),
        Some(want(&single)),
        "the offered Single segment shows its cost line",
    );
    assert_eq!(
        segment_sub_line::<ModeBurstButton>(&mut app),
        Some(want(&burst)),
        "the offered Burst segment shows its cost line",
    );
    assert_eq!(
        segment_sub_line::<ModeFullButton>(&mut app),
        None,
        "the NON-OFFERED Full segment shows NO cost line (the sub-line is cleared)",
    );
}
