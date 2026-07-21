//! The present pass: a dedicated camera that blits the offscreen capture image back onto the
//! window (GTW-764).
//!
//! With the world + UI cameras retargeted to the offscreen image ([`super::retarget`]), the
//! window would otherwise draw nothing. This present camera renders a full-window sprite of
//! that same image to the window, so a developer who focuses the window still sees the game
//! AND what the window shows equals what the capture pump reads, by construction.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

use super::target::QaCaptureTarget;

/// The render layer the present camera + the full-window blit sprite live on.
///
/// A framework plumbing const: disjoint from the world camera's layer `1` and the UI
/// camera's default layer `0`, so the present camera renders ONLY the blit sprite and the
/// game cameras never render it.
const PRESENT_LAYER: usize = 2;

/// The render order of the present camera.
///
/// A framework plumbing const written into [`Camera::order`]: above the world camera (`-1`)
/// and the UI camera (`0`), so the present pass composites LAST — after both game cameras
/// have written the offscreen image this frame.
const PRESENT_ORDER: isize = 100;

/// Marker for the present camera that blits the offscreen capture image onto the window.
///
/// Plumbing around the framework camera (exempt from no-bare-types, like [`WorldCamera`](gdtf_battle_presenter::WorldCamera)
/// / `UiCamera`), not a domain value.
#[derive(Component, Debug, Clone, Copy)]
pub(in crate::dev::net_qa) struct QaPresentCamera;

/// Marker for the full-window sprite that displays the offscreen capture image on the present
/// camera's layer. A no-bare-types unit marker.
#[derive(Component, Debug, Clone, Copy)]
struct QaPresentSprite;

/// `Update`: once the offscreen [`QaCaptureTarget`] exists and no present camera has been
/// spawned yet, spawn the present [`Camera2d`] (targeting the window at [`PRESENT_ORDER`])
/// plus a full-window [`Sprite`] of the capture image on [`PRESENT_LAYER`] — the standard
/// render-to-texture blit-back (GTW-764).
///
/// Spawns EXACTLY ONCE (guarded on the present camera's absence). The sprite is sized to the
/// window's LOGICAL size so the physical-resolution capture image fills the window at 1:1
/// under the present camera's default 2D projection (`ScalingMode::WindowSize`, 1 world unit
/// = 1 logical pixel). Window resize is NOT handled (C6 — the QA harness uses a fixed window
/// size); the sprite keeps its spawn-time size.
///
/// Param-only (`bevy-traps.md` #7): queries + the target resource + `Commands`, no
/// `&mut World`.
pub(in crate::dev::net_qa) fn spawn_present_pass(
    target: Option<Res<QaCaptureTarget>>,
    existing: Query<(), With<QaPresentCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let layers = RenderLayers::layer(PRESENT_LAYER);
    commands.spawn((
        Camera2d,
        Camera {
            order: PRESENT_ORDER,
            ..default()
        },
        RenderTarget::Window(WindowRef::Primary),
        layers.clone(),
        QaPresentCamera,
    ));
    commands.spawn((
        Sprite {
            image: (**target).clone(),
            custom_size: Some(window.size()),
            ..default()
        },
        layers,
        QaPresentSprite,
    ));
}
