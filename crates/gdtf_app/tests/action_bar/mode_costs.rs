use bevy::prelude::*;
use gdtf_app::test_support::{ModeBurstButton, ModeFullButton, ModeSingleButton};
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::mode_tu_cost,
    prelude::{Direction, StanceKind},
    tuning::CombatTuning,
    weapon::{FireMode, FireModeSpec, ModeKind},
};

use super::{harness::*, probes::*};

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
    app.update();
    app.update();

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
