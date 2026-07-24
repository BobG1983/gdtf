//! Both UI stacks are genuinely UP at the same time, in one app, in one `AppState`.
//!
//! These are the preconditions the [`clicks`](super::clicks) assertions rest on: if only one
//! stack were ever alive, an "each button moved its own tally" proof would be vacuous.

use bevy_egui::{EguiContext, EguiPlugin};
use gdtf_app::test_support::AppState;

use super::harness::{
    app_state, app_with_both_ui_stacks, bevy_ui_button_size, egui_context_entity,
};

/// One running app holds BOTH stacks: `EguiPlugin` is registered AND the `bevy_ui` button has a
/// real, non-zero computed layout — at the same time, in `AppState::Running`.
///
/// Pin-discriminating: a build where the egui half failed to register (or where adding
/// `EguiPlugin` broke `bevy_ui`'s layout) fails one of the three halves.
#[test]
fn both_ui_stacks_are_alive_in_one_running_app() {
    let mut app = app_with_both_ui_stacks();

    assert_eq!(app_state(&app), Some(AppState::Running));
    assert!(
        app.is_plugin_added::<EguiPlugin>(),
        "the spike must bring the egui stack up alongside bevy_ui",
    );
    let size = bevy_ui_button_size(&mut app);
    assert!(
        size.is_some_and(|size| size.x > 0.0 && size.y > 0.0),
        "the bevy_ui button must be laid out with a non-zero size while egui is alive; got \
         {size:?}",
    );
}

/// The primary egui context lives on the SAME camera `bevy_ui` renders through.
///
/// That shared camera is what makes "both on screen at once" true rather than accidental — an
/// egui context auto-attached to some other camera would draw somewhere `bevy_ui` does not.
#[test]
fn the_primary_egui_context_rides_the_ui_camera() -> Result<(), &'static str> {
    let mut app = app_with_both_ui_stacks();

    let context = egui_context_entity(&mut app)
        .ok_or("exactly one entity must carry the primary egui context")?;
    assert!(
        app.world().get::<EguiContext>(context).is_some(),
        "the bound entity must carry the egui context bevy_egui's required components add",
    );
    assert!(
        app.world().get::<bevy::camera::Camera2d>(context).is_some(),
        "the primary egui context must be bound to the game's UI Camera2d, not a fresh entity",
    );
    Ok(())
}
