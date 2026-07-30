//! Putting one capture on the render path: clear its output path, choose which pixels it reads,
//! and spawn the `Screenshot` that writes the PNG (GTW-880, GTW-917, GTW-918, GTW-922).
//!
//! Split out of [`super::pump`] because it changes for a different reason: the pump owns the
//! claim / settle / poll / reply machine, while this file owns WHICH RENDER TARGET a capture reads
//! and the filesystem preparation around its output file. Every editor-QA capture-source change
//! so far (GTW-917's arm mapping, GTW-918's offscreen default, GTW-922's whole-target fix) edited
//! only this concern.

use bevy::{
    camera::RenderTarget,
    image::Image,
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};
use gdtf_screenshot::CapturePath;

use super::config::EditorShotSource;

/// Delete anything already at the exact capture path, then spawn the REAL capture of whichever
/// pixels [`EditorShotSource`] names, writing to `path`.
pub(super) fn spawn_capture(
    path: &CapturePath,
    source: &EditorShotSource,
    commands: &mut Commands,
) {
    // Delete-before-spawn: a stale PNG at this exact path (a prior run's leftover at a reused
    // sequence) must never be mistaken for THIS capture's output. After the delete, the only
    // file that can appear here is the one this capture writes.
    purge_existing(path);
    let screenshot = match source {
        // The window-swapchain fallback (GTW-917). No test observes a real window readback
        // here: a cargo test thread cannot create winit's event loop on macOS, so no test app
        // has a window to read back, and a pixel assertion would pass or fail on where the
        // window happened to be. What IS covered is the choice made on this line —
        // `test/source.rs` asserts this arm spawns a
        // `Screenshot(RenderTarget::Window(WindowRef::Primary))` and the arm below spawns a
        // `Screenshot(RenderTarget::Image(..))` naming its handle. Per GTW-764 this source
        // reads back black from a backgrounded, occluded or minimized macOS window, which is
        // why GTW-918 gives the editor an offscreen render target and makes the arm below the
        // source the running editor takes.
        EditorShotSource::PrimaryWindow => Screenshot::primary_window(),
        // No offscreen target created yet: the plugin's `init_resource` installs
        // `EditorShotSource`'s `Default`, whose handle is `Handle::<Image>::default()` — the
        // `ImagePlugin::build` registers Bevy's 1x1 white `Image::default()` at
        // (`bevy_image-0.19.0/src/image.rs:220-222`). Capturing THAT would land a 1x1 white PNG
        // as if it were the editor, or fail the copy outright since its descriptor carries no
        // `COPY_SRC`. So the pump takes the fallback the game's pump takes when no target
        // resource exists (`crates/gdtf_app/src/dev/net_qa/screenshot/pump.rs:262-266`): the
        // window swapchain. Reachable only on the frames before
        // `ensure_editor_capture_target` has a sized primary window, and in a build wired
        // without `EditorCapturePresentPlugin`.
        EditorShotSource::Offscreen(target) if target.handle == Handle::<Image>::default() => {
            Screenshot::primary_window()
        }
        // The WHOLE `ImageRenderTarget` the source carries, NOT `Screenshot::image(handle)`
        // (GTW-922). That constructor's `From<Handle<Image>>` hardcodes `scale_factor: 1.0`,
        // while the editor's egui camera is aimed at the same image at the WINDOW's factor — and
        // Bevy keys each view's output attachment by the whole `ImageRenderTarget`
        // (`ViewTargetAttachments`, `bevy_render-0.19.0/src/view/mod.rs:713`; the two-field
        // `PartialEq`/`Hash`, `bevy_camera-0.19.0/src/camera.rs:983-999`). Building a second
        // value here registered the screenshot's own texture under a key no camera renders to,
        // and the copy read untouched texture memory: a black PNG from a real image. Cloning the
        // source's target keeps the two keys identical by construction.
        EditorShotSource::Offscreen(target) => Screenshot(RenderTarget::Image(target.clone())),
    };
    commands
        .spawn(screenshot)
        .observe(save_to_disk((**path).clone()));
}

/// Best-effort: create the confined screenshot directory so the capture can write into it. A
/// failure here surfaces as the poll never finding the PNG (a clean timeout), never a panic.
pub(super) fn ensure_dir(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
}

/// Best-effort: remove any pre-existing file at the exact capture path so a stale PNG can
/// never be mistaken for THIS capture's output. A `NotFound` error is the normal case (the
/// path is usually fresh) and is ignored, as is any other error — a residual file that could
/// not be removed simply keeps polling and, absent a fresh landing, times out.
fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
