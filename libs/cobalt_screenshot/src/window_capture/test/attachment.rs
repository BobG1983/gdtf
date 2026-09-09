use bevy::{
    app::SubApp,
    camera::RenderTarget,
    prelude::*,
    render::{
        RenderApp, render_asset::RenderAssets, render_resource::TextureView,
        sync_world::RenderEntity, texture::GpuImage, view::ViewTarget,
    },
    window::WindowRef,
};
use cobalt_test_utils::gpu_probe::gpu_adapter_probe;

use super::harness::{a_capture_is_in_flight, drive_until, enqueue_capture, gpu_capture_app};
use crate::window_capture::CaptureImage;

#[test]
fn a_capture_frame_aims_the_windows_output_attachment_at_the_capture_image() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP a_capture_frame_aims_the_windows_output_attachment_at_the_capture_image: no \
             usable wgpu adapter (GPU-less runner). The override runs in the render app, which a \
             backend-less harness never builds.",
        );
        return;
    }
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = gpu_capture_app(tmp.path());
    let camera = app
        .world_mut()
        .spawn((Camera2d, RenderTarget::Window(WindowRef::Primary)))
        .id();
    enqueue_capture(&mut app, "attachment");

    drive_until(&mut app, a_capture_is_in_flight);

    let Some(render_camera) = app
        .world()
        .get::<RenderEntity>(camera)
        .map(RenderEntity::id)
    else {
        unreachable!("a camera on a live backend is synced into the render world");
    };
    let Some(render_world) = app.get_sub_app(RenderApp).map(SubApp::world) else {
        unreachable!("a live backend builds the render app");
    };
    let owned = render_world.resource::<CaptureImage>();
    let wanted = render_world
        .resource::<RenderAssets<GpuImage>>()
        .get(&**owned)
        .map(|gpu_image| gpu_image.texture_view.id());
    assert!(
        wanted.is_some(),
        "test setup: the capture image must have reached the GPU before the capture frame",
    );

    let aimed_at = render_world
        .get::<ViewTarget>(render_camera)
        .and_then(ViewTarget::out_texture)
        .map(TextureView::id);
    assert_eq!(
        aimed_at, wanted,
        "on a capture frame the window camera must output into the capture image, so the frame \
         the readback reads is the one the camera drew; it found {aimed_at:?} against the capture \
         image's {wanted:?}. None means bevy dropped the camera's ViewTarget for want of an \
         output, which is a capture image nothing renders into — every capture comes back blank",
    );
}
