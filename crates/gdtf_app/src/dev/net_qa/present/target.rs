//! The offscreen capture-target [`Image`] and its one-time creation (GTW-764).
//!
//! The world + UI cameras render INTO this image every tick (regardless of window focus),
//! and the T7 capture pump reads it with `Screenshot::image` — so a capture never depends
//! on a live, focused window swapchain (the black-frame bug on a backgrounded macOS window).

use bevy::{
    image::Image,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

/// The pixel format of the offscreen capture target.
///
/// A framework layout const (the no-bare-types clause-4 plumbing carve-out): the
/// [`TextureFormat`] a render target is created with. `Rgba8UnormSrgb` is a standard
/// 2D-render target format and the byte order the T7 PNG readback expects.
const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// The offscreen render-target [`Image`] the world + UI cameras render into and the T7
/// capture pump reads via `Screenshot::image` (GTW-764).
///
/// Named-newtype [`Resource`] over `Handle<Image>` (no-bare-types); the inner is PRIVATE,
/// read through the derived [`Deref`]. Created ONCE by [`ensure_capture_target`] at the
/// primary window's physical size, with `TextureUsages::COPY_SRC` added so the capture pump
/// can copy the image out — the sole reason a capture ever lands (see the MANDATORY note in
/// [`ensure_capture_target`]).
#[derive(Resource, Debug, Clone, Deref)]
pub(in crate::dev::net_qa) struct QaCaptureTarget(Handle<Image>);

impl QaCaptureTarget {
    /// Wrap the offscreen image handle.
    const fn new(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

/// `Update`: create the offscreen [`QaCaptureTarget`] image once, sized to the primary
/// window's PHYSICAL size (GTW-764).
///
/// Registered with a `run_if(not(resource_exists::<QaCaptureTarget>))` gate, so it runs only
/// until the target exists (then never again — a fixed-size target, C6). It no-ops while no
/// primary window exists or the window reports a zero size, re-trying next frame.
///
/// [`Image::new_target_texture`] sets `TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT` but
/// NOT `COPY_SRC`; `Screenshot::image` COPIES the image out and Bevy does not auto-add that
/// usage, so the `|= COPY_SRC` below is MANDATORY — without it the capture silently never
/// lands (the pump's disk poll times out).
///
/// Param-only (`bevy-traps.md` #7): a `Query` + `ResMut<Assets<Image>>` + `Commands`, no
/// `&mut World`.
pub(in crate::dev::net_qa) fn ensure_capture_target(
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
    // MANDATORY (see the system doc): `Screenshot::image` copies from this texture, so it
    // MUST carry `COPY_SRC`; `new_target_texture` does not set it.
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let handle = images.add(image);
    commands.insert_resource(QaCaptureTarget::new(handle));
}
