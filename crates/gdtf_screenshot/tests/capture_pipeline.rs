use std::{fs, path::PathBuf};

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
use gdtf_screenshot::{
    CapturePath, CaptureProgress, ScreenshotCapturePlugin, SettleFrames, settle::PollCap,
};

const TEST_SETTLE: SettleFrames = SettleFrames::new(3);

fn headless_capture_app(path: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(
        ScreenshotCapturePlugin::with_path(CapturePath::new(path))
            .settle(TEST_SETTLE)
            .poll_cap(PollCap::new(10)),
    );
    app
}

#[test]
fn active_plugin_wires_the_pipeline_and_requests_after_settle() {
    let path = std::env::temp_dir().join("gdtf_screenshot_test_active.png");
    let mut app = headless_capture_app(path);

    assert!(
        app.world().get_resource::<CapturePath>().is_some(),
        "active plugin inserts CapturePath"
    );
    assert!(
        app.world().get_resource::<CaptureProgress>().is_some(),
        "active plugin inits CaptureProgress"
    );

    app.update();
    assert!(
        !progress_requested(&app),
        "no capture requested before the settle window"
    );

    for _ in 0..*TEST_SETTLE + 2 {
        app.update();
    }
    assert!(
        progress_requested(&app),
        "capture requested once the settle window elapsed"
    );
    assert!(
        spawned_a_screenshot(&mut app),
        "a Screenshot entity was spawned (the save_to_disk pipeline)"
    );
}

#[test]
fn inert_plugin_registers_nothing() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ScreenshotCapturePlugin::from_env(
        "GDTF_SCREENSHOT_TEST_DEFINITELY_UNSET_VAR",
    ));

    assert!(
        app.world().get_resource::<CapturePath>().is_none(),
        "inert plugin inserts no CapturePath"
    );
    assert!(
        app.world().get_resource::<CaptureProgress>().is_none(),
        "inert plugin inits no CaptureProgress"
    );

    for _ in 0..8 {
        app.update();
    }
    assert!(
        !spawned_a_screenshot(&mut app),
        "inert plugin never spawns a Screenshot"
    );
}

#[test]
fn is_active_reflects_the_path_gate() {
    assert!(
        ScreenshotCapturePlugin::with_path(CapturePath::new(PathBuf::from("/abs/out.png")))
            .is_active(),
        "a configured path is active"
    );
    assert!(
        !ScreenshotCapturePlugin::from_env("GDTF_SCREENSHOT_TEST_DEFINITELY_UNSET_VAR").is_active(),
        "an unset env var is inert"
    );
}

const TARGET_PX: u32 = 32;

const MAX_CAPTURE_UPDATES: usize = 120;

#[test]
fn real_gpu_capture_writes_a_png() {
    use gdtf_test_utils::gpu_probe::{GpuAdapterProbe, gpu_adapter_probe};

    if gpu_adapter_probe() == GpuAdapterProbe::Absent {
        eprintln!(
            "SKIP real_gpu_capture_writes_a_png: no usable wgpu adapter (GPU-less runner) — the \
             capture pipeline wiring is covered by the headless test above."
        );
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

fn progress_requested(app: &App) -> bool {
    app.world()
        .get_resource::<CaptureProgress>()
        .is_some_and(CaptureProgress::is_requested)
}

fn spawned_a_screenshot(app: &mut App) -> bool {
    let mut query = app.world_mut().query::<&Screenshot>();
    query.iter(app.world()).next().is_some()
}
