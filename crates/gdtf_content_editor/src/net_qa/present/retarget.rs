//! Retarget the editor's egui camera to the offscreen capture image (GTW-918).
//!
//! The editor's UI is egui, not `bevy_ui`, and its context is a COMPONENT ON THE CAMERA ENTITY
//! (`crate::camera::spawn_editor_camera` attaches [`PrimaryEguiContext`] explicitly, GTW-515).
//! So retargeting that camera moves egui's output with it — there is no `bevy_ui`-style default
//! camera binding to fix up, and no second egui context is created (a second multipass context
//! on the same schedule label would panic inside `bevy_egui`).

use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};

use super::target::EditorQaCaptureTarget;

/// `Update`: point the editor's egui camera at the offscreen [`EditorQaCaptureTarget`] image,
/// at the WINDOW's scale factor, but only once `bevy_egui` has recorded that camera's input
/// mapping (GTW-918).
///
/// ## Why the mapping gate exists — it keeps editor input alive
///
/// This DEPENDS ON `bevy_egui` INTERNAL BEHAVIOUR: a deliberately stale
/// [`WindowToEguiContextMap`] entry. All egui input routing (`CursorMoved`, `MouseButtonInput`,
/// `KeyboardInput`, `Ime`, wheel / zoom) goes through that resource. `bevy_egui` 0.41 populates
/// it ONLY for a WINDOW-targeted context, ONLY once, on `Added<EguiContext>`, and NEVER
/// re-evaluates it when `RenderTarget` changes — entries are removed only when the
/// `EguiContext` component is. So a camera that is image-targeted at the moment its context is
/// first observed gets NO mapping and the UI renders while being completely dead to input
/// (`egui_input.focused` also goes false, so text fields stop taking keys).
///
/// The timing window is narrow and real: `spawn_editor_camera` runs `OnEnter(Editing)`, i.e. in
/// `StateTransition`, which runs AFTER `PreUpdate` in `Main` — so `bevy_egui` only observes the
/// `Added<EguiContext>` in the NEXT frame's `PreUpdate`. A retarget in `Update` on the same
/// frame would beat it and permanently kill editor input for the process lifetime. Gating on
/// `map.context_to_window.contains_key(&entity)` makes that impossible: until the mapping is
/// recorded, this system leaves the camera window-targeted.
///
/// The resource and both of its fields are public and the behaviour is stable in 0.41, but a
/// future `bevy_egui` that refreshes the map on `Changed<RenderTarget>` would break this. The
/// documented fallback then is `FocusedNonWindowEguiContext` plus driving
/// `EguiContextPointerPosition` manually; that is NOT built now.
///
/// ## Why the scale factor is set explicitly
///
/// `impl From<Handle<Image>> for ImageRenderTarget` sets `scale_factor: 1.0`. egui takes
/// `native_pixels_per_point` from the camera's target scaling factor and derives its screen
/// rect from it, while pointer positions arrive in LOGICAL window coordinates — so on a 2x
/// display a `1.0` factor would render the UI at half apparent size in the capture AND land
/// clicks at half the correct position. The window's own scale factor is written instead.
///
/// IDEMPOTENT: a camera already aimed at the offscreen image is skipped, so the write never
/// re-dirties the camera every frame.
///
/// Param-only (`bevy-traps.md` #7): resources + filtered queries + `Commands`, no `&mut World`.
pub(in crate::net_qa) fn retarget_editor_camera_to_offscreen(
    target: Option<Res<EditorQaCaptureTarget>>,
    map: Option<Res<WindowToEguiContextMap>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(Entity, &RenderTarget), With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    let (Some(target), Some(map)) = (target, map) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let scale_factor = window.scale_factor();
    let handle = &**target;
    for (entity, current) in &cameras {
        // Already aimed at the offscreen image — nothing to do.
        if current.as_image() == Some(handle) {
            continue;
        }
        // `bevy_egui` has not yet recorded this context's window mapping; retargeting now
        // would erase editor input forever (see the system doc).
        if !map.context_to_window.contains_key(&entity) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image(ImageRenderTarget {
                handle: handle.clone(),
                scale_factor,
            }));
    }
}
