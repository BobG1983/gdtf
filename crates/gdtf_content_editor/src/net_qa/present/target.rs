//! The editor's offscreen capture-target [`Image`], its one-time creation, and the capture
//! source that names it (GTW-918).
//!
//! The editor's egui camera renders INTO this image every tick (regardless of whether the
//! editor window is visible), and the capture pump reads it with `Screenshot::image` — so a
//! capture never depends on a live, visible window swapchain.

use bevy::{
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

/// The offscreen render-target [`Image`] the editor's egui camera renders into and the capture
/// pump reads via `Screenshot::image` (GTW-918).
///
/// Named-newtype [`Resource`] over `Handle<Image>` (no-bare-types); the inner is PRIVATE, read
/// through the derived [`Deref`]. Created ONCE by [`ensure_editor_capture_target`] at the
/// primary window's physical size, with `TextureUsages::COPY_SRC` added so the capture pump can
/// copy the image out — the sole reason a capture ever lands (see the MANDATORY note in
/// [`ensure_editor_capture_target`]).
#[derive(Resource, Debug, Clone, Deref)]
pub(in crate::net_qa) struct EditorQaCaptureTarget(Handle<Image>);

impl EditorQaCaptureTarget {
    /// Wrap the offscreen image handle.
    const fn new(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

/// `Update`: create the offscreen [`EditorQaCaptureTarget`] image once, sized to the primary
/// window's PHYSICAL size, and point [`EditorShotSource`] at it (GTW-918).
///
/// Registered with a `run_if(not(resource_exists::<EditorQaCaptureTarget>))` gate, so it runs
/// only until the target exists (then never again — a fixed-size target). It no-ops while no
/// primary window exists or the window reports a zero size, re-trying next frame. A headless
/// editor test app has no window at all, so nothing is created there and a caller-pinned
/// capture source is left alone.
///
/// [`Image::new_target_texture`] sets `TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT` but NOT
/// `COPY_SRC`; `Screenshot::image` COPIES the image out and Bevy does not auto-add that usage,
/// so the `|= COPY_SRC` below is MANDATORY — without it the capture silently never lands (the
/// pump's disk poll times out). `test/present.rs`'s
/// `the_capture_target_is_window_sized_rgba_with_copy_src` fails if that line is deleted.
///
/// The [`EditorShotSource::Offscreen`] insert is what makes clause 1 true at runtime: the
/// plugin's `init_resource` installs the enum's [`Default`], an `Offscreen` carrying the
/// placeholder `Handle::<Image>::default()` (which names Bevy's registered 1x1
/// `Image::default()`, so the pump captures the window rather than it — see the [`Default`] impl
/// on [`EditorShotSource`]), and this replaces it with the target actually being rendered into.
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
    // MANDATORY (see the system doc): `Screenshot::image` copies from this texture, so it MUST
    // carry `COPY_SRC`; `new_target_texture` does not set it.
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let handle = images.add(image);
    commands.insert_resource(EditorShotSource::Offscreen(handle.clone()));
    commands.insert_resource(EditorQaCaptureTarget::new(handle));
}
