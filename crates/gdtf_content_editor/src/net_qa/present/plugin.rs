//! [`EditorCapturePresentPlugin`] — wires the editor's offscreen-capture present path
//! (GTW-918).

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use super::{
    blit::spawn_editor_present_pass,
    retarget::retarget_editor_camera_to_offscreen,
    target::{EditorQaCaptureTarget, ensure_editor_capture_target},
};
use crate::net_qa::schedule::EditorNetQaSystems;

/// The DEV-ONLY plugin that retargets the editor's egui camera to an offscreen image and blits
/// it back to the window, so the capture pump reads pixels the render graph writes EVERY tick —
/// independent of whether the editor window is visible (GTW-918, the editor's version of the
/// game's GTW-764 answer).
///
/// Added ONLY on [`NetQaEditorPlugin`](crate::net_qa::NetQaEditorPlugin)'s listener arm (its
/// `serve` wiring) and by this module's tests; an inert plugin, a non-`net_qa` build and a
/// release editor never see it, so their render path is exactly as before — no offscreen
/// target, no present camera, no [`WinitSettings`] override.
pub(in crate::net_qa) struct EditorCapturePresentPlugin;

impl Plugin for EditorCapturePresentPlugin {
    fn build(&self, app: &mut App) {
        // Render a fresh offscreen frame every tick regardless of focus, so a capture never
        // races the window's reactive-low-power throttle when the editor is unfocused. Gated
        // behind `net_qa` (this plugin is only added on the env-active path), so normal dev /
        // release power behaviour is unchanged.
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
        // `Update`, chained — NEVER `EguiPrimaryContextPass` (bevy-traps #8: the egui pass
        // schedule can run TWICE per frame under multipass, and creating a render target /
        // spawning a camera twice a frame is not idempotent in a useful way). Ordered: create
        // the target first, then retarget the egui camera and spawn the present pass (both read
        // the target). `insert_resource` is deferred, so the target is visible from the NEXT
        // frame — a one-frame settle at startup, harmless.
        //
        // All three sit in `EditorNetQaSystems::Present`, and that band is ordered BEFORE
        // `Gather`, where the capture pump reads the egui camera's `RenderTarget` to check that
        // the pixels a capture would read are the pixels that camera writes. Without this edge
        // the pump's read and the retarget's deferred write are ambiguous (bevy-traps #3), so a
        // screenshot settling on the retarget frame would be captured or refused depending on
        // how the executor happened to order them; the edge is also what makes Bevy insert the
        // sync point that applies the retarget's `Commands` before the pump runs. Configured
        // HERE, not at the `Gather` registration, so the constraint travels with the systems it
        // constrains. An app that adds this plugin and registers no `Gather` system is
        // unaffected — an empty set has no systems to order.
        app.configure_sets(
            Update,
            EditorNetQaSystems::Present.before(EditorNetQaSystems::Gather),
        );
        app.add_systems(
            Update,
            (
                ensure_editor_capture_target.run_if(not(resource_exists::<EditorQaCaptureTarget>)),
                retarget_editor_camera_to_offscreen,
                spawn_editor_present_pass,
            )
                .chain()
                .in_set(EditorNetQaSystems::Present),
        );
    }
}
