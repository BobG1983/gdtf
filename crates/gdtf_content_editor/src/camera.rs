//! The editor's standalone camera spawn (GTW-512 C1.2; GTW-515: explicit primary egui context).
//!
//! The pre-egui `bevy_ui` shell spawned the `Camera2d` inside `spawn_editor_shell`. That shell is
//! gone (the egui swap), so the camera moves to its own tiny `OnEnter(Editing)` system. egui still
//! needs a camera rendering to the window (its render target), and the screenshot capture needs a
//! `Camera2d` to read back the primary window.
//!
//! ## GTW-515: explicit primary egui context (the two-camera disambiguation)
//!
//! Before GTW-515 the editor had ONE camera, so `bevy_egui`'s `auto_create_primary_context` correctly
//! attached the [`PrimaryEguiContext`] to it. GTW-515 adds a SECOND camera — the offscreen prefab
//! preview camera (rendering to an `Image`, not the window). `bevy_egui`'s auto-create attaches the
//! primary context to the FIRST `Added<Camera>` it sees (regardless of render target — an ambiguous
//! ordering, bevy-traps #3); if that races to the preview camera, egui draws into the OFFSCREEN
//! image and the window goes blank. So the editor now DISABLES auto-create
//! ([`disable_egui_auto_context`], a `Startup` system) and EXPLICITLY attaches
//! [`PrimaryEguiContext`] to THIS window camera — deterministic, never the preview camera.

use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};

/// `Startup`: DISABLE `bevy_egui`'s `auto_create_primary_context` so the primary egui context is
/// never auto-attached to the (ambiguously-first) camera — the editor attaches it EXPLICITLY to the
/// window camera in [`spawn_editor_camera`] instead (GTW-515). Runs at `Startup`, before any camera
/// spawns (the cameras spawn `OnEnter(Editing)`), which is the window `bevy_egui` requires the flag be
/// set in.
pub(crate) fn disable_egui_auto_context(mut settings: ResMut<EguiGlobalSettings>) {
    settings.auto_create_primary_context = false;
}

/// `OnEnter(Editing)`: spawn the editor's own 2D window camera (the editor never reuses a game
/// camera), carrying the [`PrimaryEguiContext`] EXPLICITLY (GTW-515 — so egui draws into the window,
/// never the offscreen preview camera's `Image` target).
///
/// The camera is spawned WINDOW-targeted, and that is load-bearing beyond the ordinary case
/// (GTW-918). `bevy_egui` records this context's entry in `WindowToEguiContextMap` — the resource
/// every keyboard / pointer / IME event is routed through — once, on `Added<EguiContext>`, and only
/// for a WINDOW-targeted context. Spawning image-targeted would record nothing and leave the editor
/// UI rendering while dead to input. So the QA present path retargets this camera to its offscreen
/// image LATER, and only after that mapping exists.
///
/// Which means: on the `net_qa` listener arm, egui renders into the QA capture image rather than
/// straight at the window (a present camera blits that image back onto the window, so the window
/// still shows the editor). Everywhere else — a normal editor launch, a release build — egui
/// renders against this camera's window target and the QA capture reads the window back.
pub(crate) fn spawn_editor_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PrimaryEguiContext));
}
