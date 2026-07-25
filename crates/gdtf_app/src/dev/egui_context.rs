//! Binds the dev affordances' primary egui context to the game's captured UI camera
//! (GTW-732; re-homed from the procgen stepper to `crate::dev` by GTW-864).
//!
//! `bevy_egui` auto-attaches the primary egui context to the FIRST camera an app spawns
//! (`EguiGlobalSettings::auto_create_primary_context`, on by default). Two reasons that is
//! wrong here:
//!
//! 1. **Render-graph ordering against `bevy_ui`.** egui renders in the render graph of the
//!    camera that carries its context, ordered against `bevy_ui`'s own pass on that camera
//!    (`bevy_egui`'s `UiRenderOrder`). The game's HUD is `bevy_ui` drawn through
//!    [`UiCamera`], so putting the egui context on that SAME camera is what makes a dev
//!    overlay draw over the game UI by construction rather than by accident of whichever
//!    camera egui happened to land on.
//! 2. **Headless capture.** Under the DEV `net_qa` capture path (GTW-764) the
//!    offscreen-present camera spawns first and targets the WINDOW, so an auto-attached
//!    overlay renders to a window a headless QA run never presents — absent from the
//!    offscreen capture image the world + UI cameras render into, and therefore invisible in
//!    every wire screenshot. [`UiCamera`] is exactly the camera the `net_qa` present path
//!    retargets to the offscreen capture image, so an overlay bound there lands in the
//!    captured pixels.
//!
//! So [`super::plugin`] disables auto-attach where it adds `EguiPlugin`, and binds the primary
//! context to the persistent [`UiCamera`] here. In a plain (non-`net_qa`) dev run that camera
//! still renders to the window, so the panel shows on screen exactly as before. Either way the
//! context is pinned to a known, persistent, captured camera instead of whichever camera happens
//! to spawn first.
//!
//! The binding lives beside the `EguiPlugin` add because the two are ONE decision: turning
//! auto-attach off is what makes an explicit binding owed. GTW-864 moved both up here from the
//! procgen stepper, which had made that call back when it was the only egui-using affordance.

use bevy::prelude::*;
use bevy_egui::PrimaryEguiContext;

use crate::states::running::UiCamera;

/// `Update`: attach the primary egui context to the game's [`UiCamera`] as soon as it exists
/// (spawned `OnEnter(AppState::Running)`), once.
///
/// Idempotent via the `Without<PrimaryEguiContext>` filter — the marker is not a required
/// component, so its absence is a correct "not bound yet" signal; once bound the query is empty
/// and this no-ops. Inserting [`PrimaryEguiContext`] pulls its required `EguiContext` (and, since
/// `bevy_egui`'s multipass resource is present, the `EguiPrimaryContextPass` schedule a dev panel
/// draws in) via `bevy_egui`'s own required-component wiring.
///
/// Param-only (`bevy-traps.md` #7): a filtered `Query` + `Commands`, no `&mut World`.
pub(super) fn bind_primary_egui_context(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<PrimaryEguiContext>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(PrimaryEguiContext);
        info!("dev-affordances: primary egui context bound to the captured UI camera");
    }
}
