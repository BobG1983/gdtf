//! Put the captured frame back on the window's swapchain.

use bevy::{
    core_pipeline::blit::{BlitPipeline, BlitPipelineKey},
    ecs::system::SystemParam,
    prelude::*,
    render::{
        gpu_readback::Readback,
        render_asset::RenderAssets,
        render_resource::{
            CommandEncoderDescriptor, LoadOp, Operations, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor, SpecializedRenderPipelines, StoreOp,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        view::ExtractedWindows,
    },
};

use super::{attachment::reads, target::CaptureImage};

/// Device, queue and pipeline cache the blit pass draws through.
#[derive(SystemParam)]
pub(super) struct BlitGpu<'w> {
    device:    Res<'w, RenderDevice>,
    queue:     Res<'w, RenderQueue>,
    cache:     Res<'w, PipelineCache>,
    blit:      Option<Res<'w, BlitPipeline>>,
    pipelines: ResMut<'w, SpecializedRenderPipelines<BlitPipeline>>,
}

// Draws the capture image over the swapchain the capture frame did not write.
pub(super) fn blit_capture_to_window(
    capture: Option<Res<CaptureImage>>,
    readbacks: Query<&Readback>,
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<GpuImage>>,
    mut gpu: BlitGpu,
) {
    let Some(capture) = capture else {
        return;
    };
    if !readbacks.iter().any(|readback| reads(readback, &capture)) {
        return;
    }
    let Some(window) = windows.primary.and_then(|entity| windows.get(&entity)) else {
        return;
    };
    let (Some(view), Some(format)) = (
        window.swap_chain_texture_view.as_ref(),
        window.swap_chain_texture_view_format,
    ) else {
        return;
    };
    let Some(gpu_image) = images.get(&**capture) else {
        return;
    };
    let Some(blit) = gpu.blit.as_ref() else {
        return;
    };
    let key = BlitPipelineKey {
        target_format: format,
        blend_state:   None,
        samples:       1,
        source_space:  None,
    };
    let id = gpu.pipelines.specialize(&gpu.cache, blit, key);
    let Some(pipeline) = gpu.cache.get_render_pipeline(id) else {
        return;
    };
    let bind_group = blit.create_bind_group(&gpu.device, &gpu_image.texture_view, &gpu.cache);
    let mut encoder = gpu
        .device
        .create_command_encoder(&CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label:                    Some("gdtf_screenshot_capture_blit"),
            color_attachments:        &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load:  LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes:         None,
            occlusion_query_set:      None,
            multiview_mask:           None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    gpu.queue.submit([encoder.finish()]);
}
