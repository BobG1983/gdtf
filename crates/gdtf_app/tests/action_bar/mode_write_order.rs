//! A mode picked on the panel lives on the gun, so re-selecting the shooter cannot undo it.

use bevy::prelude::*;
use gdtf_app::test_support::ModeBurstButton;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    prelude::{Direction, StanceKind},
    weapon::ModeKind,
};
use gdtf_test_utils::press_ui_button;

use super::{harness::*, probes::*};

/// A battle whose selected shooter holds a single/burst/full gun, on that gun's own single.
fn battle_on_the_guns_single_mode() -> (App, Entity) {
    let mut app = battle_running_app();
    let shooter = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();
    assert_eq!(
        selected_mode(&app),
        Some(spec(ModeKind::Single, 0.2, 1)),
        "the case starts on the gun's own single spec, which is what a gun with no mode picked \
         for it reads as",
    );
    (app, shooter)
}

#[test]
fn a_panel_picked_mode_outlasts_re_selecting_the_shooter_on_the_same_frame() {
    let (mut app, shooter) = battle_on_the_guns_single_mode();

    let Some(burst_segment) = require_button::<ModeBurstButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, burst_segment);
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(spec(ModeKind::Burst, 0.4, 3)),
        "the mode lives on the gun, so re-selecting the shooter on the frame the panel wrote \
         leaves it on Burst; the gun's single here means the selection put it back",
    );
}

#[test]
fn a_panel_picked_mode_is_still_the_one_the_next_frame_prices() {
    let (mut app, _shooter) = battle_on_the_guns_single_mode();

    let Some(burst_segment) = require_button::<ModeBurstButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, burst_segment);
    app.update();
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(spec(ModeKind::Burst, 0.4, 3)),
        "every later price reads the gun on a later frame, so the mode has to still be Burst \
         once the frame that set it is over",
    );
}
