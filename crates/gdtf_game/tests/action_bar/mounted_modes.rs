//! While a mount is wielded, the mode panel offers, prices and picks the MOUNT's modes.

use bevy::{ecs::entity::Entity, prelude::*, ui::Display};
use cobalt_test_utils::press_ui_button;
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::mode_tu_cost,
    prelude::{Direction, StanceKind},
    tuning::CombatTuning,
    weapon::{FireMode, FireModeSpec, ModeKind, MountedWeapon, WieldedBy},
};
use gdtf_game::test_support::{ModeBurstButton, ModeFullButton, ModeSingleButton};

use super::{harness::*, probes::*};

/// The TU the shooter's pool is sized at, so a mode's percentage lands on a readable number.
const POOL: TuMax = TuMax::new(100);

/// Frames run after each step, so every panel system has had its chance to react.
const SETTLE: usize = 3;

fn settle(app: &mut App) {
    for _ in 0..SETTLE {
        app.update();
    }
}

/// Wield a mounted weapon offering `modes` on `ganger`, as manning an emplacement does.
fn mount_on(app: &mut App, ganger: Entity, modes: FireMode) -> Entity {
    let mount = app
        .world_mut()
        .spawn((WieldedBy::new(ganger), modes, MountedWeapon))
        .id();
    settle(app);
    mount
}

/// Despawn the mount, as leaving an emplacement does.
fn dismount(app: &mut App, mount: Entity) {
    app.world_mut().entity_mut(mount).despawn();
    settle(app);
}

/// A selected shooter holding a gun offering `carried`, manning a mount offering `mount`.
fn armed_over_a_mount(
    app: &mut App,
    carried: &[FireModeSpec],
    mount: &[FireModeSpec],
) -> (Entity, Entity) {
    let ganger = arm_and_select_with_tu(
        app,
        FireMode::new(carried.to_vec()),
        POOL,
        StanceKind::Standing,
        Direction::North,
    );
    // The selection settles first: in play a ganger is picked many frames before it mans a mount.
    settle(app);
    // Wielded AFTER the carried gun, which is the order entering an emplacement produces.
    let mount = mount_on(app, ganger, FireMode::new(mount.to_vec()));
    (ganger, mount)
}

#[test]
fn the_mode_panel_shows_the_mounted_weapons_modes_not_the_carried_guns() {
    let mut app = battle_running_app();
    armed_over_a_mount(
        &mut app,
        &[
            spec(ModeKind::Single, 0.2, 1),
            spec(ModeKind::Burst, 0.4, 3),
        ],
        &[spec(ModeKind::Full, 0.7, 6)],
    );

    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::Flex),
        "the MOUNT offers Full, so the Full segment must be shown",
    );
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::None),
        "Single is offered by the CARRIED gun alone, so the panel must hide it while the mount \
         is what fires",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::None),
        "Burst is offered by the CARRIED gun alone, so the panel must hide it too",
    );
}

#[test]
fn the_mode_cost_line_prices_the_mounted_weapons_spec() {
    let mut app = battle_running_app();
    let carried_full = spec(ModeKind::Full, 0.3, 6);
    let mount_full = spec(ModeKind::Full, 0.8, 6);
    armed_over_a_mount(&mut app, &[carried_full], &[mount_full]);

    let tuning = CombatTuning::default();
    let hip = Aiming::new(false);
    let priced = |mode: &FireModeSpec| format!("{} TU", *mode_tu_cost(mode, &POOL, &hip, &tuning));
    assert_ne!(
        priced(&mount_full),
        priced(&carried_full),
        "the case only means something while the two weapons charge different TU for Full",
    );
    assert_eq!(
        segment_sub_line::<ModeFullButton>(&mut app),
        Some(priced(&mount_full)),
        "the Full cost line must be the MOUNT's charge, not the carried gun's",
    );
}

#[test]
fn selecting_and_clicking_a_mode_both_resolve_against_the_mounted_weapon() {
    let mut app = battle_running_app();
    let mount_single = spec(ModeKind::Single, 0.5, 1);
    let mount_full = spec(ModeKind::Full, 0.8, 6);
    armed_over_a_mount(
        &mut app,
        &[spec(ModeKind::Single, 0.2, 1), spec(ModeKind::Full, 0.3, 6)],
        &[mount_single, mount_full],
    );

    assert_eq!(
        selected_mode(&app),
        Some(mount_single),
        "arming the mount must put the selection on the MOUNT's Single spec, not the carried \
         gun's",
    );

    let Some(full_segment) = require_button::<ModeFullButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, full_segment);
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(mount_full),
        "clicking the Full segment must resolve against the MOUNT, so the panel never highlights \
         a spec the fired weapon does not hold",
    );
}

#[test]
fn the_highlighted_segment_is_one_the_panel_still_shows() {
    let mut app = battle_running_app();
    armed_over_a_mount(
        &mut app,
        &[
            spec(ModeKind::Single, 0.2, 1),
            spec(ModeKind::Burst, 0.4, 3),
        ],
        &[spec(ModeKind::Full, 0.7, 6)],
    );

    assert!(
        segment_is_active::<ModeFullButton>(&mut app),
        "the MOUNT offers Full alone, so the highlight must sit on the Full segment",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::Flex),
        "the highlighted segment must be one the panel shows; a highlight on a hidden segment \
         leaves the player no mode button that works",
    );
}

#[test]
fn leaving_the_mount_puts_the_panel_and_the_selection_back_on_the_carried_gun() {
    let mut app = battle_running_app();
    let carried_burst = spec(ModeKind::Burst, 0.4, 3);
    let mount_full = spec(ModeKind::Full, 0.7, 6);
    let (_, mount) = armed_over_a_mount(&mut app, &[carried_burst], &[mount_full]);
    assert_eq!(
        selected_mode(&app),
        Some(mount_full),
        "the case only means something once the selection has moved to the MOUNT's spec",
    );

    dismount(&mut app, mount);

    assert_eq!(
        selected_mode(&app),
        Some(carried_burst),
        "leaving the mount must put the selection back on the carried gun's spec, or the next \
         shot fires the carried gun with the mount's mode",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::Flex),
        "the carried gun offers Burst, so the panel must show it again once the mount is gone",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::None),
        "Full was the MOUNT's mode alone, so the panel must hide it again once the mount is gone",
    );
}
