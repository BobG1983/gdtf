//! [`CapturePresentPlugin`] — wires the GTW-764 offscreen-capture present path.

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use super::{
    blit::spawn_present_pass,
    retarget::{mark_ui_default_camera, retarget_cameras_to_offscreen},
    target::{QaCaptureTarget, ensure_capture_target},
};

/// The DEV-ONLY plugin that retargets the game cameras to an offscreen image and blits it
/// back to the window, so the T7 capture pump reads pixels the render graph writes EVERY
/// tick — independent of window focus / occlusion (GTW-764).
///
/// Added ONLY on the env-active `net_qa` path (`NetQaPlugin`'s listener arm) and by the
/// GTW-764 present tests; a non-`net_qa` / release build never sees it, so the window
/// swapchain render path is unchanged there (no offscreen target, no present pass, no
/// `WinitSettings` override).
pub(in crate::dev::net_qa) struct CapturePresentPlugin;

impl Plugin for CapturePresentPlugin {
    fn build(&self, app: &mut App) {
        // C5: render a fresh offscreen frame every tick regardless of focus, so a capture
        // never races the window's reactive-low-power throttle when the window is
        // unfocused. Gated behind `net_qa` (this plugin is only added on the env-active
        // path), so normal dev / release power behaviour is unchanged.
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
        // Ordered: create the target first, then retarget the cameras + spawn the present
        // pass (both read the target). `insert_resource` is deferred, so the target is
        // visible from the NEXT frame — a one-frame settle at startup, harmless.
        app.add_systems(
            Update,
            (
                ensure_capture_target.run_if(not(resource_exists::<QaCaptureTarget>)),
                retarget_cameras_to_offscreen,
                // Bind the HUD to the offscreen UI camera — must run so the present camera
                // (added below) never becomes the implicit default UI camera (C-fix GTW-764).
                mark_ui_default_camera,
                spawn_present_pass,
            )
                .chain(),
        );
    }
}
