//! Binds the stepper's primary egui context to the game's captured UI camera (GTW-732).
//!
//! `bevy_egui` auto-attaches the primary egui context to the FIRST camera an app spawns
//! (`EguiGlobalSettings::auto_create_primary_context`, on by default). Under the DEV `net_qa`
//! capture path (GTW-764) the offscreen-present camera spawns first and targets the WINDOW, so
//! the auto-attached schematic renders to a window a headless QA run never presents — absent
//! from the offscreen capture image the world + UI cameras render into, and therefore invisible
//! in every wire screenshot.
//!
//! So the stepper disables auto-attach ([`super::plugin`] flips the setting) and binds the
//! primary context to the persistent [`UiCamera`] here. That camera is exactly the one the
//! `net_qa` present path retargets to the offscreen capture image, so the schematic lands in the
//! captured pixels; in a plain (non-`net_qa`) dev run the UI camera still renders to the window,
//! so the panel shows on screen exactly as before. Either way the stepper — which OWNS where its
//! own overlay renders — pins the context to a known, persistent, captured camera instead of
//! whichever camera happens to spawn first.

use bevy::prelude::*;
use bevy_egui::PrimaryEguiContext;

use crate::states::running::UiCamera;

/// `Update`, registered only while the stepper is enabled: attach the primary egui context to
/// the game's [`UiCamera`] as soon as it exists (spawned `OnEnter(AppState::Running)`), once.
///
/// Idempotent via the `Without<PrimaryEguiContext>` filter — the marker is not a required
/// component, so its absence is a correct "not bound yet" signal; once bound the query is empty
/// and this no-ops. Inserting [`PrimaryEguiContext`] pulls its required `EguiContext` (and, since
/// `bevy_egui`'s multipass resource is present, the `EguiPrimaryContextPass` schedule the panel
/// draws in) via `bevy_egui`'s own required-component wiring.
///
/// Param-only (`bevy-traps.md` #7): a filtered `Query` + `Commands`, no `&mut World`.
pub(super) fn bind_primary_egui_context(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<PrimaryEguiContext>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(PrimaryEguiContext);
        info!("procgen-stepper: primary egui context bound to the captured UI camera");
    }
}
