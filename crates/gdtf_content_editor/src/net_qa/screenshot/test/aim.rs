//! The pre-spawn consistency check (GTW-922): an offscreen capture whose target the editor's UI
//! camera is not rendering into is REFUSED with a typed reply, never answered with a blank PNG.
//!
//! Every test here drives the REAL pump — the same claim / settle / spawn path production uses —
//! and reads the reply off the real responder channel. Only the resources the editor's own plugin
//! would have installed are placed by hand, which is the fact under test in each case.

use bevy::{
    asset::Assets,
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
};
use bevy_egui::PrimaryEguiContext;
use gdtf_qa_protocol::{
    envelope::{QaResponse, ScreenshotResult},
    ids::ShotName,
};

use super::{
    super::config::EditorShotSource,
    support::{DRIVE_UPDATES, enqueue, pump_app, spawned_captures},
};
use crate::net_qa::present::EditorQaCaptureTarget;

/// The scale factor the fixtures use, deliberately not `1.0` (see `source.rs`'s own note).
const FIXTURE_SCALE_FACTOR: f32 = 2.5;

/// Add a real offscreen target to `app` and install it as BOTH the present path's
/// [`EditorQaCaptureTarget`] and the pump's [`EditorShotSource`] — the pair the running editor's
/// `ensure_editor_capture_target` installs together.
fn install_present_target(app: &mut App) -> ImageRenderTarget {
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let target = ImageRenderTarget {
        handle,
        scale_factor: FIXTURE_SCALE_FACTOR,
    };
    app.insert_resource(EditorQaCaptureTarget::new(target.clone()));
    app.insert_resource(EditorShotSource::Offscreen(target.clone()));
    target
}

/// Drive frames until the pump either spawns a capture or answers, and report the reply it sent
/// (or [`None`] if it spawned a capture instead of replying).
fn reply_or_capture(
    app: &mut App,
    rx: &std::sync::mpsc::Receiver<QaResponse>,
) -> Option<QaResponse> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Ok(reply) = rx.try_recv() {
            return Some(reply);
        }
        if spawned_captures(app) > 0 {
            return None;
        }
    }
    None
}

/// A capture is REFUSED, with a message, when the present path created a target but the camera
/// holding the primary egui context is still WINDOW-targeted.
///
/// This is the state a retarget that never fires leaves behind, and the state that produced a
/// pure-black PNG live. No `Screenshot` may be spawned, and the reply must be the typed
/// `TargetNotRendered` naming both render targets — an outcome an agent cannot mistake for a
/// screenshot of the editor.
#[test]
fn a_capture_of_an_unrendered_target_is_refused_with_a_message() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    install_present_target(&mut app);
    // An editor-shaped camera left aimed at the window: the retarget never fired.
    app.world_mut()
        .spawn((PrimaryEguiContext, RenderTarget::default()));

    let reply_rx = enqueue(&mut app, ShotName::new("unrendered".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        matches!(
            reply,
            Some(QaResponse::Screenshot(ScreenshotResult::TargetNotRendered(
                _
            )))
        ),
        "a capture of an offscreen target the UI camera does not render into must be refused as \
         TargetNotRendered, never spawned and never answered Saved; got {reply:?}",
    );
    assert!(
        spawned_captures(&mut app) == 0,
        "no Screenshot may be spawned for a refused capture — a spawned one lands a blank PNG",
    );
    let detail = match reply {
        Some(QaResponse::Screenshot(ScreenshotResult::TargetNotRendered(detail))) => {
            detail.as_str().to_owned()
        }
        _ => String::new(),
    };
    assert!(
        detail.contains("Window") && detail.contains("Image"),
        "the refusal must name BOTH render targets so a caller can see the mismatch; it said \
         {detail:?}",
    );
}

/// The check does NOT fire when the UI camera IS aimed at the target: the capture is spawned as
/// before.
///
/// Without this the check would be indistinguishable from disabling offscreen capture.
#[test]
fn a_capture_of_the_rendered_target_is_spawned() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = install_present_target(&mut app);
    app.world_mut()
        .spawn((PrimaryEguiContext, RenderTarget::Image(target)));

    let reply_rx = enqueue(&mut app, ShotName::new("rendered".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        reply.is_none() && spawned_captures(&mut app) > 0,
        "with the UI camera aimed at the capture target the pump must spawn the capture and not \
         refuse it; it replied {reply:?}",
    );
}

/// A camera aimed at the right IMAGE at the wrong SCALE FACTOR is refused too.
///
/// That combination is the GTW-922 defect itself: two `ImageRenderTarget` values sharing a handle
/// are DIFFERENT render targets to Bevy, so the capture would still read a texture nothing drew
/// into. A check that compared handles alone would pass here and the black frame would come back.
#[test]
fn a_camera_at_the_wrong_scale_factor_is_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = install_present_target(&mut app);
    app.world_mut().spawn((
        PrimaryEguiContext,
        RenderTarget::Image(ImageRenderTarget::from(target.handle)),
    ));

    let reply_rx = enqueue(&mut app, ShotName::new("wrong_scale".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        matches!(
            reply,
            Some(QaResponse::Screenshot(ScreenshotResult::TargetNotRendered(
                _
            )))
        ),
        "a UI camera aimed at the capture image at a DIFFERENT scale factor renders into a \
         different render target, so the capture must be refused; got {reply:?}",
    );
}

/// With NO present path in the app, a caller-inserted offscreen source is captured as before.
///
/// The check is scoped to the present path on purpose: absent an
/// [`EditorQaCaptureTarget`], nothing ever claimed a camera renders into the named image, and the
/// pump's own GPU-free tests plus any windowless editor app drive exactly that arrangement. The
/// GPU integration test is not one of them — its app has a real primary window, so the target
/// exists there and the check runs live.
#[test]
fn a_caller_pinned_offscreen_source_without_a_present_path_is_still_captured() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(EditorShotSource::Offscreen(ImageRenderTarget::from(handle)));

    let reply_rx = enqueue(&mut app, ShotName::new("no_present_path".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        reply.is_none() && spawned_captures(&mut app) > 0,
        "with no present path the check must not refuse a caller-pinned offscreen capture; it \
         replied {reply:?}",
    );
}
