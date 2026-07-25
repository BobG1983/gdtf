//! Binds the primary egui context to the game's persistent UI camera.
//!
//! `bevy_egui` auto-attaches the primary egui context to the FIRST camera an app spawns
//! (`EguiGlobalSettings::auto_create_primary_context`, on by default). Two reasons that is
//! wrong here:
//!
//! 1. **Coexistence.** egui renders in the render graph of the camera that carries its
//!    context, ordered against `bevy_ui`'s own pass on that camera. Putting the context on
//!    the camera `bevy_ui` already draws through is what lets the two stacks share one
//!    screen — and, for this harness, what makes a swap a change of DRAWING rather than a
//!    change of which camera you are looking at.
//! 2. **Capture.** Under the DEV `net_qa` path (GTW-764) the offscreen-present camera
//!    spawns first and targets the WINDOW, so an auto-attached context renders where a
//!    headless QA run never looks: an agent that swapped to the egui stack and took a
//!    screenshot would get a frame with neither stack in it. This binding is what makes the
//!    swap visible to the wire QA path at all.
//!
//! Same mechanism, and the same reason, as the procgen stepper's own context binding.

use bevy::prelude::*;
use bevy_egui::PrimaryEguiContext;

use crate::states::running::UiCamera;

/// `Update`: attach the primary egui context to the game's [`UiCamera`] as soon as it
/// exists (spawned `OnEnter(AppState::Running)`), once.
///
/// Idempotent via the `Without<PrimaryEguiContext>` filter — once bound the query is empty
/// and this no-ops, which also makes it safe for the other `dev_tools` affordances to run
/// their own identical binding alongside it. Inserting [`PrimaryEguiContext`] pulls its
/// required `EguiContext` (and the `EguiPrimaryContextPass` schedule the panel draws in)
/// through `bevy_egui`'s own required-component wiring.
///
/// Param-only (`bevy-traps.md` #7): a filtered `Query` + [`Commands`].
pub(super) fn bind_primary_egui_context(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<PrimaryEguiContext>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(PrimaryEguiContext);
        info!("ui-swap: primary egui context bound to the UI camera");
    }
}
