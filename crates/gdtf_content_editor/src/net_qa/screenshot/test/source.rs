//! Which pixels a capture reads: the [`EditorShotSource`] arm → spawned-[`Screenshot`] target
//! mapping, and the source the editor's own plugin installs (GTW-917).
//!
//! Before this file the two arms of [`EditorShotSource`] were EXECUTED by the other pump tests
//! (through the [`Default`]) but never OBSERVED: no test read the spawned [`Screenshot`]'s
//! render target, so swapping the arms — or replacing either with the other's constructor —
//! left the suite green. These three tests close that hole:
//!
//! 1. [`the_primary_window_arm_captures_the_primary_window_swapchain`] — the
//!    [`PrimaryWindow`](EditorShotSource::PrimaryWindow) arm spawns a
//!    `Screenshot(RenderTarget::Window(WindowRef::Primary))`.
//! 2. [`the_offscreen_arm_captures_the_image_it_names`] — the
//!    [`Offscreen`](EditorShotSource::Offscreen) arm spawns a
//!    `Screenshot(RenderTarget::Image(..))` naming THAT handle, not merely some image.
//! 3. [`the_plugin_installs_the_shipped_capture_source`] — an app carrying
//!    [`NetQaEditorPlugin`] and nothing else ends up with the source the shipped editor uses,
//!    so a silent flip of the enum's `#[default]` is caught.
//! 4. [`the_default_placeholder_source_falls_back_to_the_primary_window`] — the
//!    [`Offscreen`](EditorShotSource::Offscreen) arm still carrying the [`Default`]'s
//!    placeholder handle captures the WINDOW, never Bevy's registered 1x1 `Image::default()`
//!    (GTW-918).
//!
//! [`Screenshot`] is `pub struct Screenshot(pub RenderTarget)` in `bevy_render` 0.19, so this
//! is a pure-ECS read that needs no render device — the same assertion the game's GTW-764
//! coverage makes (`crates/gdtf_app/src/dev/net_qa/present/test/capture_source.rs`).
//!
//! What these tests do NOT prove: that a REAL editor window's swapchain reads back the egui
//! shell rather than a black frame. See the module doc of `super::super` for the full
//! standing-in ledger.

use bevy::{
    asset::Assets, camera::RenderTarget, image::Image, prelude::*,
    render::view::window::screenshot::Screenshot, window::WindowRef,
};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::ids::ShotName;

use super::{
    super::config::EditorShotSource,
    support::{drive_until_spawned, enqueue, pump_app},
};
use crate::net_qa::NetQaEditorPlugin;

/// The render target of the single spawned [`Screenshot`], or [`None`] unless there is exactly
/// one — the capture the pump spawned for the one request these tests queue.
fn spawned_screenshot_target(app: &mut App) -> Option<RenderTarget> {
    let mut shots = app.world_mut().query::<&Screenshot>();
    let targets: Vec<RenderTarget> = shots.iter(app.world()).map(|shot| shot.0.clone()).collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// Drive the REAL pump on `app` through one request under `source` and hand back the render
/// target of the capture it spawned.
///
/// The whole production path runs: the router-shaped enqueue, the advance-before-claim
/// ordering, the settle window, and the delete-before-spawn purge. Only the source resource is
/// pinned, which is the fact under test.
fn capture_target_under(
    app: &mut App,
    source: EditorShotSource,
    name: &str,
) -> Option<RenderTarget> {
    app.insert_resource(source);
    // Held to the end of the test: dropping the reply channel would disconnect the responder
    // while the capture the assertion reads is still in flight.
    let _reply_rx = enqueue(app, ShotName::new(name.to_owned()));
    drive_until_spawned(app)?;
    spawned_screenshot_target(app)
}

/// The [`PrimaryWindow`](EditorShotSource::PrimaryWindow) arm captures the PRIMARY WINDOW's
/// swapchain: the spawned [`Screenshot`] carries `RenderTarget::Window(WindowRef::Primary)`.
///
/// Fails if the arm is swapped with [`Offscreen`](EditorShotSource::Offscreen), or if its
/// `Screenshot::primary_window()` is replaced by `Screenshot::image(..)`.
#[test]
fn the_primary_window_arm_captures_the_primary_window_swapchain() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = capture_target_under(&mut app, EditorShotSource::PrimaryWindow, "window_arm");
    assert!(
        matches!(target, Some(RenderTarget::Window(WindowRef::Primary))),
        "EditorShotSource::PrimaryWindow must spawn a capture of the primary window's \
         swapchain (RenderTarget::Window(WindowRef::Primary)), got {target:?}",
    );
}

/// The [`Offscreen`](EditorShotSource::Offscreen) arm captures the IMAGE IT NAMES: the spawned
/// [`Screenshot`] carries `RenderTarget::Image(..)` holding that exact handle.
///
/// The handle identity is asserted, not just the variant, so replacing the arm's
/// `image.clone()` with any other handle (`Handle::default()` included) fails here. Fails, too,
/// if the two arms are swapped: a primary-window capture carries no image at all.
#[test]
fn the_offscreen_arm_captures_the_image_it_names() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    // A real, distinct handle from a real asset collection — a shared `Handle::default()`
    // would compare equal to any other default and prove nothing about which image was named.
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());

    let target = capture_target_under(
        &mut app,
        EditorShotSource::Offscreen(handle.clone()),
        "offscreen_arm",
    );

    assert!(
        matches!(target, Some(RenderTarget::Image(_))),
        "EditorShotSource::Offscreen must spawn a capture of an offscreen image \
         (RenderTarget::Image(..)), got {target:?}",
    );
    let named = target.as_ref().and_then(RenderTarget::as_image);
    assert!(
        named == Some(&handle),
        "EditorShotSource::Offscreen must capture the image handle it carries; the spawned \
         capture named {named:?}",
    );
}

/// The [`Default`]'s PLACEHOLDER handle is not captured: an
/// [`Offscreen`](EditorShotSource::Offscreen) source still carrying `Handle::<Image>::default()`
/// spawns a capture of the primary window's swapchain, not of an image.
///
/// Why this matters (GTW-918): `Handle::<Image>::default()` names Bevy's REGISTERED 1x1 white
/// `Image::default()` (`ImagePlugin::build`, `bevy_image-0.19.0/src/image.rs:220-222`) — not "no
/// image". Capturing it would land a 1x1 white PNG as if it were the editor's shell, or fail the
/// copy because that image carries no `COPY_SRC`. The pump takes the window swapchain instead,
/// the same fallback the game's pump takes with no capture target
/// (`crates/gdtf_app/src/dev/net_qa/screenshot/pump.rs:262-266`).
///
/// Fails if `spawn_capture`'s placeholder guard is deleted: the capture then names
/// `RenderTarget::Image(..)`.
#[test]
fn the_default_placeholder_source_falls_back_to_the_primary_window() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = capture_target_under(
        &mut app,
        EditorShotSource::Offscreen(Handle::<Image>::default()),
        "placeholder_source",
    );
    assert!(
        matches!(target, Some(RenderTarget::Window(WindowRef::Primary))),
        "an Offscreen source carrying the Default's placeholder handle must capture the primary \
         window's swapchain rather than Bevy's registered 1x1 Image::default(), got {target:?}",
    );
}

/// An app carrying [`NetQaEditorPlugin`] and nothing else ends up with the capture source the
/// SHIPPED editor uses.
///
/// The value is the point, not the constant: `NetQaEditorPlugin` is the only thing in the
/// editor binary that installs an [`EditorShotSource`] (via `init_resource`, so it installs
/// the enum's [`Default`]), and this assertion pins what that installs. GTW-918 flipped it from
/// [`PrimaryWindow`](EditorShotSource::PrimaryWindow) to
/// [`Offscreen`](EditorShotSource::Offscreen), so the running editor can never fall back to the
/// window swapchain by accident. A SILENT flip in EITHER direction is impossible.
///
/// This app is `MinimalPlugins`, so it has no window for the present path to size a target
/// against and the handle here is the [`Default`]'s placeholder. That the RUNNING editor's
/// source names the image its camera actually renders into is a separate, stronger assertion —
/// `tests/net_qa_editor_present` makes it on the real editor app.
#[test]
fn the_plugin_installs_the_shipped_capture_source() {
    // `listening` rather than `from_env`: the env gate cannot be driven from a test (the
    // workspace forbids `unsafe`, and `set_var` is unsafe in edition 2024), and an inert
    // plugin would install nothing at all. Port 0 lets the OS pick a free port.
    let Ok((plugin, _port)) = NetQaEditorPlugin::listening(NetQaPort::new(0)) else {
        unreachable!("binding a loopback listener on an OS-assigned port succeeds");
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(plugin);

    let source = app.world().get_resource::<EditorShotSource>();
    assert!(
        matches!(source, Some(EditorShotSource::Offscreen(_))),
        "NetQaEditorPlugin must install the capture source the shipped editor captures \
         through — EditorShotSource::Offscreen since GTW-918, never PrimaryWindow; it \
         installed {source:?}",
    );
}
