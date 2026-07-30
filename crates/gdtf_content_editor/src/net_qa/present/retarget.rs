//! Retarget the editor's egui camera to the offscreen capture image (GTW-918).
//!
//! The editor's UI is egui, not `bevy_ui`, and its context is a COMPONENT ON THE CAMERA ENTITY
//! (`crate::camera::spawn_editor_camera` attaches [`PrimaryEguiContext`] explicitly, GTW-515).
//! So retargeting that camera moves egui's output with it — there is no `bevy_ui`-style default
//! camera binding to fix up, and no second egui context is created (a second multipass context
//! on the same schedule label would panic inside `bevy_egui`).

use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};

use super::target::{EditorQaCaptureTarget, aims_at};

/// `Update`: point the editor's egui camera at the offscreen [`EditorQaCaptureTarget`] — the
/// image AND the scale factor it carries — but only once `bevy_egui` has recorded that camera's
/// input mapping (GTW-918, GTW-922).
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
/// ## Why the whole target comes from the resource
///
/// `impl From<Handle<Image>> for ImageRenderTarget` sets `scale_factor: 1.0`, and egui needs the
/// WINDOW's factor (see [`EditorQaCaptureTarget`]'s doc for what egui derives from it). This
/// system used to build its own `ImageRenderTarget` from the handle plus a freshly-read window
/// scale factor, while the capture pump built a second one from the handle alone. Bevy keys a
/// view's output attachment by the whole `ImageRenderTarget`, both fields included, so the two
/// values addressed DIFFERENT render targets and every capture read a texture nothing drew into
/// — a black PNG (GTW-922). The single [`EditorQaCaptureTarget`] value is now the only
/// `ImageRenderTarget` in the editor's QA path, and both readers clone it.
///
/// IDEMPOTENT: a camera already aimed at exactly that target is skipped, so the write never
/// re-dirties the camera every frame. The comparison is on the WHOLE target rather than on the
/// handle, so a camera aimed at the right image at the wrong scale factor is corrected instead of
/// being left as-is.
///
/// Param-only (`bevy-traps.md` #7): resources + a filtered query + `Commands`, no `&mut World`.
pub(in crate::net_qa) fn retarget_editor_camera_to_offscreen(
    target: Option<Res<EditorQaCaptureTarget>>,
    map: Option<Res<WindowToEguiContextMap>>,
    cameras: Query<(Entity, &RenderTarget), With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    let (Some(target), Some(map)) = (target, map) else {
        return;
    };
    for (entity, current) in &cameras {
        // Already aimed at the offscreen target — nothing to do.
        if aims_at(current, &target) {
            continue;
        }
        // `bevy_egui` has not yet recorded this context's window mapping; retargeting now
        // would erase editor input forever (see the system doc).
        if !map.context_to_window.contains_key(&entity) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image((**target).clone()));
    }
}
