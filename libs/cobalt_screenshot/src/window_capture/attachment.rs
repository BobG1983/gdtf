//! Point the window's output attachment at the capture image for a capture frame.

use bevy::{
    prelude::*,
    render::{
        gpu_readback::Readback,
        render_asset::RenderAssets,
        texture::{GpuImage, OutputColorAttachment},
        view::ViewTargetAttachments,
    },
};

use super::target::{CaptureImage, CaptureWindowTarget};

// The attachments resource is cleared every frame, so this re-inserts on every capture frame.
pub(super) fn override_window_attachment(
    capture: Option<Res<CaptureImage>>,
    aimed_at: Option<Res<CaptureWindowTarget>>,
    readbacks: Query<&Readback>,
    images: Res<RenderAssets<GpuImage>>,
    mut attachments: ResMut<ViewTargetAttachments>,
) {
    let (Some(capture), Some(aimed_at)) = (capture, aimed_at) else {
        return;
    };
    if !readbacks.iter().any(|readback| reads(readback, &capture)) {
        return;
    }
    let Some(gpu_image) = images.get(&**capture) else {
        return;
    };
    attachments.insert(
        (**aimed_at).clone(),
        OutputColorAttachment::new(gpu_image.texture_view.clone(), gpu_image.view_format()),
    );
}

// Whether this readback is the one a capture queued on the capture image.
pub(super) fn reads(readback: &Readback, capture: &CaptureImage) -> bool {
    matches!(readback, Readback::Texture(handle) if handle == &**capture)
}
