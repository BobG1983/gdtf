//!    so a silent flip of the enum's `#[default]` is caught.
use bevy::{
    asset::Assets,
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
    render::view::window::screenshot::Screenshot,
    window::WindowRef,
};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::ids::ShotName;

use super::{
    super::config::EditorShotSource,
    support::{drive_until_spawned, enqueue, pump_app},
};
use crate::net_qa::NetQaEditorPlugin;

fn spawned_screenshot_target(app: &mut App) -> Option<RenderTarget> {
    let mut shots = app.world_mut().query::<&Screenshot>();
    let targets: Vec<RenderTarget> = shots.iter(app.world()).map(|shot| shot.0.clone()).collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

fn capture_target_under(
    app: &mut App,
    source: EditorShotSource,
    name: &str,
) -> Option<RenderTarget> {
    app.insert_resource(source);
    let _reply_rx = enqueue(app, ShotName::new(name.to_owned()));
    drive_until_spawned(app)?;
    spawned_screenshot_target(app)
}

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

const FIXTURE_SCALE_FACTOR: f32 = 3.0;

#[test]
fn the_offscreen_arm_captures_the_whole_render_target_it_names() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let wanted = ImageRenderTarget {
        handle,
        scale_factor: FIXTURE_SCALE_FACTOR,
    };

    let target = capture_target_under(
        &mut app,
        EditorShotSource::Offscreen(wanted.clone()),
        "offscreen_arm",
    );

    assert!(
        matches!(target, Some(RenderTarget::Image(_))),
        "EditorShotSource::Offscreen must spawn a capture of an offscreen image \
         (RenderTarget::Image(..)), got {target:?}",
    );
    let named = match &target {
        Some(RenderTarget::Image(image)) => Some(image.clone()),
        _ => None,
    };
    assert!(
        named.as_ref() == Some(&wanted),
        "EditorShotSource::Offscreen must capture the whole ImageRenderTarget it carries — handle \
         AND scale factor {FIXTURE_SCALE_FACTOR}, since Bevy keys a view's output attachment by \
         both. The spawned capture named {named:?}",
    );
}

#[test]
fn the_default_placeholder_source_falls_back_to_the_primary_window() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = capture_target_under(
        &mut app,
        EditorShotSource::Offscreen(ImageRenderTarget::from(Handle::<Image>::default())),
        "placeholder_source",
    );
    assert!(
        matches!(target, Some(RenderTarget::Window(WindowRef::Primary))),
        "an Offscreen source carrying the Default's placeholder handle must capture the primary \
         window's swapchain rather than Bevy's registered 1x1 Image::default(), got {target:?}",
    );
}

#[test]
fn the_plugin_installs_the_shipped_capture_source() {
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
