use std::path::Path;

use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
};
use cobalt_test_utils::gpu_probe::gpu_adapter_probe;

use super::harness::{enqueue_capture, gpu_capture_app};
use crate::{
    capture::{CaptureCompletions, CaptureOutcome},
    settle::PollCap,
    window_capture::CaptureImage,
};

/// Colour the test camera clears the capture image to.
const LIT: Color = Color::srgb(0.9, 0.2, 0.4);

/// A per-capture readback poll cap no run reaches, so the readback waits out any load.
const GPU_POLL_BUDGET: u32 = u32::MAX;

const WINDOW_PX: UVec2 = UVec2::new(320, 180);

// One drained completion, or none yet this frame.
fn next_outcome(app: &mut App) -> Option<CaptureOutcome> {
    let mut completions = app.world_mut().resource_mut::<CaptureCompletions<()>>();
    let finished = completions.drain();
    let [one] = finished.as_slice() else {
        return None;
    };
    Some(one.outcome().clone())
}

// Every RGB triple in the PNG, so a black frame is countable rather than guessed at.
fn dark_pixels(png: &Path) -> Result<(UVec2, usize), String> {
    let bytes = std::fs::read(png).map_err(|err| format!("reading {}: {err}", png.display()))?;
    let decoded =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).map_err(|err| {
            format!(
                "{} bytes at {} do not decode: {err}",
                bytes.len(),
                png.display()
            )
        })?;
    let rgb = decoded.to_rgb8();
    let size = UVec2::new(rgb.width(), rgb.height());
    let dark = rgb
        .pixels()
        .filter(|pixel| pixel.0.iter().all(|channel| *channel == 0))
        .count();
    Ok((size, dark))
}

#[test]
fn a_capture_of_a_rendered_frame_writes_a_png_that_is_not_black() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP a_capture_of_a_rendered_frame_writes_a_png_that_is_not_black: no usable wgpu \
             adapter (GPU-less runner). Nothing renders and nothing reads back without a device.",
        );
        return;
    }
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = gpu_capture_app(tmp.path());
    app.insert_resource(PollCap::new(GPU_POLL_BUDGET));
    app.update();

    // camera_driver skips a window camera with no extracted window, which only winit supplies.
    let image = app.world().resource::<CaptureImage>().clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(LIT),
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle:       (*image).clone(),
            scale_factor: 1.0,
        }),
    ));
    enqueue_capture(&mut app, "rendered_frame");

    let png = loop {
        app.update();
        if let Some(CaptureOutcome::Landed(path)) = next_outcome(&mut app) {
            break (*path).clone();
        }
    };

    let found = dark_pixels(&png);
    let Ok((size, dark)) = found else {
        unreachable!("the landed capture must decode as a PNG: {found:?}")
    };
    assert_eq!(
        size, WINDOW_PX,
        "the capture must come back at the window's own physical size, because the image the \
         frame is rendered into is built from Window::physical_size",
    );
    assert_eq!(
        dark,
        0,
        "a capture of a frame the GPU really drew must carry that frame's pixels: {dark} of {} \
         are pure black. All of them black means the readback read a texture nothing rendered \
         into, or the writer turned the bytes into zeroes",
        size.x * size.y,
    );
}
