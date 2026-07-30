//! The pre-spawn consistency check: never capture an offscreen target the editor's UI camera is
//! not rendering into (GTW-922).
//!
//! ## Why a check exists at all
//!
//! An offscreen capture of an image nothing draws into is not an error anywhere in Bevy. The
//! image is real, its descriptor is right, the `COPY_SRC` usage is there, the copy succeeds, the
//! PNG lands and decodes — and every pixel is zero. The reply says `Saved`, an agent reads a
//! black frame as the editor's screen, and QA passes on nothing. GTW-918 shipped exactly that
//! state and GTW-922 found it live.
//!
//! So the pump asks one question before it spawns a capture: is the camera drawing the editor's
//! UI actually aimed at the target this capture would read? If it is not, no capture is spawned
//! and the client is answered
//! [`TargetNotRendered`](gdtf_qa_protocol::envelope::ScreenshotResult::TargetNotRendered) naming
//! both render targets. A typed refusal is auditable; a black PNG is not.
//!
//! ## When the read happens, relative to the write
//!
//! The [`RenderTarget`] this check reads is written by the present path's
//! `retarget_editor_camera_to_offscreen` through [`Commands`]. The pump runs in
//! [`EditorNetQaSystems::Gather`](crate::net_qa::EditorNetQaSystems), which is ordered AFTER
//! `EditorNetQaSystems::Present` (where the retarget runs) — so on the frame the retarget fires,
//! this check reads the target the camera was just aimed at rather than the stale window target.
//! Without that ordering the two are ambiguous (bevy-traps #3) and a shot settling on the
//! retarget frame is captured or refused at random; `test/ordering.rs` drives exactly that frame.
//!
//! ## What the check does NOT claim
//!
//! It is scoped to the present path. When no
//! [`EditorQaCaptureTarget`](crate::net_qa::present::EditorQaCaptureTarget) exists, the editor
//! has no offscreen present path in this configuration and a caller-inserted
//! [`EditorShotSource::Offscreen`] makes no claim that any camera renders into it — that is the
//! arrangement the pump's own GPU-free tests use, and the arrangement of any windowless editor app
//! (`ensure_editor_capture_target` needs a primary window to size the image), so refusing there
//! would only reject captures that legitimately land. The GPU integration test is NOT in that
//! arrangement: its app has a real primary window, so the target does exist and this check runs
//! live there — which makes that test a guard against a wrongly-refusing check too, since it
//! demands a saved PNG. The window arm and the placeholder handle (which the pump answers with a
//! window capture) make no offscreen claim either.

use bevy::{camera::RenderTarget, ecs::system::SystemParam, prelude::*};
use bevy_egui::PrimaryEguiContext;
use gdtf_qa_protocol::envelope::CaptureAimNet;

use super::config::EditorShotSource;
use crate::net_qa::present::{EditorQaCaptureTarget, aims_at};

/// The answer to "may this capture be spawned?".
///
/// A named two-case enum rather than a `bool` or a bare `Option<String>` (no-bare-types): the
/// cases carry the domain meaning "the pixels this capture reads are the pixels the UI camera
/// writes" and "they are not, and here is what was found instead".
pub(in crate::net_qa) enum CaptureAim {
    /// The capture may proceed — either it makes no offscreen claim, or the UI camera is aimed
    /// at exactly the target it would read.
    Confirmed,
    /// Refused before spawning: the message names what the capture would read against what the
    /// UI camera is actually rendering into.
    Refused(CaptureAimNet),
}

/// What the editor's UI camera is rendering into, read at the moment a capture would be spawned.
///
/// A [`SystemParam`] so the pump takes it as one argument and stays under clippy's
/// argument-count ceiling, the same bundling its tunables use.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorCaptureAim<'w, 's> {
    /// The offscreen target the present path created, absent in a build or test app with no
    /// present path.
    target:  Option<Res<'w, EditorQaCaptureTarget>>,
    /// The render target of every camera carrying the primary egui context — the editor's UI
    /// camera is the only one (`crate::camera::spawn_editor_camera`, GTW-515).
    cameras: Query<'w, 's, &'static RenderTarget, With<PrimaryEguiContext>>,
}

impl EditorCaptureAim<'_, '_> {
    /// Decide whether a capture through `source` may be spawned.
    ///
    /// [`Confirmed`](CaptureAim::Confirmed) unless the present path created a target AND the
    /// source names it AND no camera holding the primary egui context is aimed at it. The
    /// comparison goes through [`aims_at`], so the image handle AND the scale factor must both
    /// match — a camera aimed at the right image at the wrong scale factor writes a different
    /// render target from the one a capture of `source` reads, which is the GTW-922 defect
    /// itself.
    pub(in crate::net_qa) fn verify(&self, source: &EditorShotSource) -> CaptureAim {
        // A window capture makes no claim about an offscreen image.
        let EditorShotSource::Offscreen(wanted) = source else {
            return CaptureAim::Confirmed;
        };
        // No present path in this app: nothing ever promised a camera renders into this image.
        let Some(target) = self.target.as_ref() else {
            return CaptureAim::Confirmed;
        };
        // The source still names the `Default`'s placeholder, which the pump answers with a
        // window capture rather than an image capture — see `pump::spawn_capture`.
        if *wanted != ***target {
            return CaptureAim::Confirmed;
        }
        if self.cameras.iter().any(|current| aims_at(current, wanted)) {
            return CaptureAim::Confirmed;
        }
        let found: Vec<String> = self
            .cameras
            .iter()
            .map(|current| format!("{current:?}"))
            .collect();
        CaptureAim::Refused(CaptureAimNet::new(format!(
            "the capture would read the offscreen target {wanted:?}, but the camera holding the \
             editor's primary egui context renders into {}. Nothing draws into that image, so \
             the PNG would be a blank frame.",
            if found.is_empty() {
                "nothing — no camera holds that context".to_owned()
            } else {
                found.join(", ")
            }
        )))
    }
}
