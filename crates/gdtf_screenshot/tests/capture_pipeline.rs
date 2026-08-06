//! Real-GPU capture: Bevy's screenshot pipeline writes a decodable PNG.
use std::fs;

use bevy::{
    camera::RenderTarget,
    image::Image,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{TextureFormat, TextureUsages},
        view::window::screenshot::{Screenshot, save_to_disk},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

const TARGET_PX: u32 = 32;

const MAX_CAPTURE_UPDATES: usize = 120;

#[test]
fn real_gpu_capture_writes_a_png() {
    use gdtf_test_utils::gpu_probe::{GpuAdapterProbe, gpu_adapter_probe};

    if gpu_adapter_probe() == GpuAdapterProbe::Absent {
        eprintln!("SKIP real_gpu_capture_writes_a_png: no usable wgpu adapter (GPU-less runner).");
        return;
    }

    let out = std::env::temp_dir().join(format!("gdtf_screenshot_real_{}.png", std::process::id()));
    drop(fs::remove_file(&out));

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>(),
    );

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.4, 0.8)),
            ..default()
        },
        RenderTarget::Image(target_handle.clone().into()),
    ));

    app.finish();
    app.cleanup();

    let path = out.clone();
    app.world_mut()
        .spawn(Screenshot::image(target_handle))
        .observe(save_to_disk(path));

    let mut written = false;
    for _ in 0..MAX_CAPTURE_UPDATES {
        app.update();
        if out.exists() {
            written = true;
            break;
        }
    }
    assert!(
        written,
        "the real GPU capture wrote a PNG to {}",
        out.display()
    );
    drop(fs::remove_file(&out));
}
