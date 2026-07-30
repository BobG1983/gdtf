//! The present pass: a dedicated camera that blits the editor's offscreen capture image back
//! onto the window (GTW-918).
//!
//! With the egui camera retargeted to the offscreen image ([`super::retarget`]), the window
//! would otherwise draw nothing. This present camera renders a full-window sprite of that same
//! image to the window, so a developer who focuses the editor still sees its UI AND what the
//! window shows equals what the capture pump reads, by construction.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

use super::target::EditorQaCaptureTarget;

/// The render layer the present camera + the full-window blit sprite live on.
///
/// A framework plumbing const: disjoint from the editor camera's default layer `0` AND from the
/// prefab preview camera's layer `1` (`crate::preview::target::PREVIEW_LAYER`), so the present
/// camera renders ONLY the blit sprite, the editor camera never renders that sprite, and the
/// preview target is untouched.
const PRESENT_LAYER: usize = 2;

/// The render order of the present camera.
///
/// A framework plumbing const written into [`Camera::order`]: above the editor camera (`0`) and
/// the preview camera (`-1`), so the present pass composites LAST — after the egui pass has
/// written the offscreen image this frame.
const PRESENT_ORDER: isize = 100;

/// Marker for the present camera that blits the offscreen capture image onto the editor window.
///
/// Plumbing around the framework camera (exempt from no-bare-types, like the editor's own
/// `PreviewCamera`), not a domain value.
#[derive(Component, Debug, Clone, Copy)]
pub(in crate::net_qa) struct EditorQaPresentCamera;

/// Marker for the full-window sprite that displays the offscreen capture image on the present
/// camera's layer. A no-bare-types unit marker.
#[derive(Component, Debug, Clone, Copy)]
struct EditorQaPresentSprite;

/// `Update`: once the offscreen [`EditorQaCaptureTarget`] exists and no present camera has been
/// spawned yet, spawn the present [`Camera2d`] (targeting the window at [`PRESENT_ORDER`]) plus
/// a full-window [`Sprite`] of the capture image on [`PRESENT_LAYER`] — the standard
/// render-to-texture blit-back (GTW-918, mirroring the game's GTW-764 pass).
///
/// Spawns EXACTLY ONCE (guarded on the present camera's absence). The sprite is sized to the
/// window's LOGICAL size so the physical-resolution capture image fills the window at 1:1 under
/// the present camera's default 2D projection (1 world unit = 1 logical pixel). Window resize is
/// NOT handled (the QA harness uses a fixed window size); the sprite keeps its spawn-time size.
///
/// The present camera carries NO egui context — `bevy_egui`'s auto-create is disabled for the
/// editor (`crate::camera::disable_egui_auto_context`, GTW-515), so spawning a second camera
/// here can never steal the primary egui context from the editor camera.
///
/// Param-only (`bevy-traps.md` #7): queries + the target resource + `Commands`, no `&mut World`.
pub(in crate::net_qa) fn spawn_editor_present_pass(
    target: Option<Res<EditorQaCaptureTarget>>,
    existing: Query<(), With<EditorQaPresentCamera>>,
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
        EditorQaPresentCamera,
    ));
    commands.spawn((
        Sprite {
            image: (**target).clone(),
            custom_size: Some(window.size()),
            ..default()
        },
        layers,
        EditorQaPresentSprite,
    ));
}
