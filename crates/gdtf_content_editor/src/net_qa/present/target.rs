//! The editor's offscreen capture target — the [`Image`] plus the scale factor it is rendered
//! at — its one-time creation, and the capture source that names it (GTW-918, GTW-922).
//!
//! The editor's egui camera renders INTO this target every tick (regardless of whether the
//! editor window is visible), and the capture pump reads it with a `Screenshot` naming the SAME
//! target — so a capture never depends on a live, visible window swapchain.

use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

use crate::net_qa::screenshot::EditorShotSource;

/// The pixel format of the editor's offscreen capture target.
///
/// A framework layout const (the no-bare-types plumbing carve-out): the [`TextureFormat`] a
/// render target is created with. `Rgba8UnormSrgb` is the format the game's GTW-764 target
/// uses, and the one `bevy_egui`'s extract picks for a non-HDR view — so the egui pass writes
/// this target with no format conversion, and the PNG readback sees the byte order it expects.
const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// The offscreen render target the editor's egui camera renders into and the capture pump reads
/// (GTW-918).
///
/// Named-newtype [`Resource`] over [`ImageRenderTarget`] — the image handle AND the scale factor
/// it is rendered at, together — not over a bare `Handle<Image>`. That pairing is the whole
/// point of the type, and it is what GTW-922 fixed.
///
/// ## Why the scale factor travels WITH the handle
///
/// Bevy keys every view's render-world output attachment by the FULL `NormalizedRenderTarget`,
/// in `ViewTargetAttachments` (`HashMap<NormalizedRenderTarget, OutputColorAttachment>`,
/// `bevy_render-0.19.0/src/view/mod.rs:713`), and `ImageRenderTarget`'s `PartialEq` / `Hash`
/// compare BOTH of its fields (`bevy_camera-0.19.0/src/camera.rs:983-999`).
/// `prepare_screenshots` registers a screenshot's own output texture under the SCREENSHOT's
/// target key (`bevy_render-0.19.0/src/view/window/screenshot.rs:309-323`) and then copies THAT
/// texture out.
///
/// So a capture built as `Screenshot::image(handle)` — whose `From<Handle<Image>>` hardcodes
/// `scale_factor: 1.0` (`bevy_camera-0.19.0/src/camera.rs:1028-1035`) — registers its texture
/// under a DIFFERENT key from a camera aimed at the same handle at scale factor `2.0`. No camera
/// renders into the screenshot's texture, and the copy reads untouched texture memory: a fully
/// black PNG out of a real, correctly-created, `COPY_SRC`-carrying image. That is what GTW-922
/// observed live on a 2x display, and why the two safety mechanisms GTW-918 shipped (the
/// placeholder fallback and the `COPY_SRC` proof) did not cover it.
///
/// Holding ONE [`ImageRenderTarget`] value that both the retarget and the capture read makes the
/// two keys the same by construction, so the mismatch cannot come back.
///
/// The inner is PRIVATE, read through the derived [`Deref`]. Created ONCE by
/// [`ensure_editor_capture_target`] at the primary window's physical size and scale factor, with
/// `TextureUsages::COPY_SRC` added so the capture pump can copy the image out — the sole reason
/// a capture ever lands (see the MANDATORY note in [`ensure_editor_capture_target`]).
#[derive(Resource, Debug, Clone, Deref)]
pub(in crate::net_qa) struct EditorQaCaptureTarget(ImageRenderTarget);

impl EditorQaCaptureTarget {
    /// Wrap the offscreen image handle and the scale factor it is rendered at.
    pub(in crate::net_qa) const fn new(target: ImageRenderTarget) -> Self {
        Self(target)
    }
}

/// Whether `current` aims at exactly `target`: the same image AND the same scale factor.
///
/// BOTH fields, always — that is the assertion the whole GTW-922 fix rests on. `RenderTarget`
/// itself has no `PartialEq` in Bevy 0.19, and comparing only the handle is precisely the
/// too-weak check that let a 1.0-versus-2.0 divergence read as "already aimed there". The three
/// callers — the retarget's idempotence skip, the present pass's gate, and the capture pump's
/// consistency check — all go through here so none of them can weaken it alone.
pub(in crate::net_qa) fn aims_at(current: &RenderTarget, target: &ImageRenderTarget) -> bool {
    matches!(current, RenderTarget::Image(image) if image == target)
}

/// `Update`: create the offscreen [`EditorQaCaptureTarget`] once, sized to the primary window's
/// PHYSICAL size and carrying the window's own scale factor, then point [`EditorShotSource`] at
/// it (GTW-918, GTW-922).
///
/// Registered with a `run_if(not(resource_exists::<EditorQaCaptureTarget>))` gate, so it runs
/// only until the target exists (then never again — a fixed-size target). It no-ops while no
/// primary window exists or the window reports a zero size, re-trying next frame. A headless
/// editor test app has no window at all, so nothing is created there and a caller-pinned
/// capture source is left alone.
///
/// [`Image::new_target_texture`] sets `TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT` but NOT
/// `COPY_SRC`; a `Screenshot` COPIES the image out and Bevy does not auto-add that usage, so the
/// `|= COPY_SRC` below is MANDATORY — without it the capture silently never lands (the pump's
/// disk poll times out). `test/present.rs`'s
/// `the_capture_target_is_window_sized_rgba_with_copy_src` fails if that line is deleted.
///
/// ## Why the WINDOW's scale factor, and why it is written HERE
///
/// egui takes `native_pixels_per_point` from the camera's target scaling factor
/// (`bevy_egui-0.41.0/src/input.rs:1211`) and derives its screen rect from it, while pointer
/// positions arrive in LOGICAL window coordinates. `ImageRenderTarget`'s `From<Handle<Image>>`
/// hardcodes `1.0` (`bevy_camera-0.19.0/src/camera.rs:1028-1035`), so the window's own factor has
/// to be written explicitly — GTW-918's reason for setting it, and the reason it is set ONCE,
/// here, with both the retarget and the capture taking it from this resource rather than each
/// constructing an [`ImageRenderTarget`] of their own. Live on a 2x display the window reports
/// `scale_factor = 2.0` with a 1280x720 logical / 2560x1440 physical size, and the target, the
/// camera and the capture all carry that same `2.0` (GTW-922, observed through the QA tools).
///
/// The [`EditorShotSource::Offscreen`] insert is what makes an offscreen capture real at
/// runtime: the plugin's `init_resource` installs the enum's [`Default`], an `Offscreen`
/// carrying the placeholder `Handle::<Image>::default()` (which names Bevy's registered 1x1
/// white `Image::default()`, so the pump captures the window rather than it — see the
/// [`Default`] impl on [`EditorShotSource`]), and this replaces it with the target actually
/// being rendered into.
///
/// Param-only (`bevy-traps.md` #7): a `Query` + `ResMut<Assets<Image>>` + `Commands`, no
/// `&mut World`.
pub(in crate::net_qa) fn ensure_editor_capture_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    let mut image = Image::new_target_texture(size.x, size.y, CAPTURE_FORMAT, None);
    // MANDATORY (see the system doc): a `Screenshot` copies from this texture, so it MUST carry
    // `COPY_SRC`; `new_target_texture` does not set it.
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = ImageRenderTarget {
        handle:       images.add(image),
        scale_factor: window.scale_factor(),
    };
    commands.insert_resource(EditorShotSource::Offscreen(target.clone()));
    commands.insert_resource(EditorQaCaptureTarget::new(target));
}
