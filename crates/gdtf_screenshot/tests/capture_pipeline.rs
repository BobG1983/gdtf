//! Integration tests for the `gdtf_screenshot` capture pipeline (GTW-510).
//!
//! Two tiers, matching the crate's dev-QA discipline:
//!
//! 1. **Headless pipeline-wiring** (always runs): a no-renderer app with the plugin active drives
//!    past the settle window and asserts the pipeline is wired — the [`CaptureProgress`] flips to
//!    `requested` (the `Screenshot` spawn + `save_to_disk` observer fired), which is observable
//!    WITHOUT a GPU / a written PNG. Also asserts the inert-by-default contract (no path -> no
//!    resources) and that `run_if(resource_exists::<CapturePath>)` keeps the systems dormant.
//! 2. **Real-PNG capture** (GPU-guarded, GTW-527): builds a real render app, spawns a
//!    `Screenshot`, and asserts a PNG lands on disk — skipped gracefully (log + return) on a
//!    GPU-less runner so it never panics inside `app.finish()`.

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

/// A tiny settle window so the headless test does not have to spin many frames.
const TEST_SETTLE: SettleFrames = SettleFrames::new(3);

/// Build a minimal headless app (no renderer) with the capture plugin active for `path`, a short
/// settle, and `MinimalPlugins` so `Update` runs. `AppExit` is a `Message` the app registers by
/// default; the poll-then-exit system writes it but we assert BEFORE the app would exit.
fn headless_capture_app(path: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // `AppExit` message + the input needed by the plugin's Update systems come from these; the
    // Screenshot spawn is a no-op with no renderer (the entity is spawned, its observer never
    // fires without a GPU readback — exactly the pipeline-wiring boundary the test asserts).
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

    // Resources present the moment the active plugin builds.
    assert!(
        app.world().get_resource::<CapturePath>().is_some(),
        "active plugin inserts CapturePath"
    );
    assert!(
        app.world().get_resource::<CaptureProgress>().is_some(),
        "active plugin inits CaptureProgress"
    );

    // Before the settle window elapses, no request has fired.
    app.update();
    assert!(
        !progress_requested(&app),
        "no capture requested before the settle window"
    );

    // Advance past the settle window; the settle_then_capture system spawns the Screenshot and
    // flips the flag — the observable pipeline-wired signal (no GPU needed).
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
    // `from_env` on an env var that is (essentially certainly) unset -> inert.
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

    // Driving it never spawns a Screenshot.
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

/// The offscreen render-target edge (square). Small keeps the readback + PNG encode cheap.
const TARGET_PX: u32 = 32;

/// Max `app.update()` calls to wait for the async GPU readback to flush the PNG to disk.
const MAX_CAPTURE_UPDATES: usize = 120;

/// The real-PNG capture proof: on a real GPU, `Screenshot::image(target)` + `save_to_disk` writes
/// an actual PNG to disk (the agent-readable artifact the whole ticket is about).
///
/// It captures a Camera2d's OFFSCREEN render target (`Screenshot::image`), NOT the primary window
/// (`Screenshot::primary_window`) — a windowed app cannot be driven from a cargo-test thread on
/// macOS (winit's `EventLoop` must be created on the main thread). The offscreen path exercises the
/// SAME crate imports (`Screenshot` + `save_to_disk`) + the real GPU readback + the real PNG encode,
/// with no window. GPU-guarded per GTW-527: on a GPU-less runner it logs a skip and returns BEFORE
/// building the render app, so it never panics inside `app.finish()`.
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

    // A render-capable headless app (no window): real wgpu device, Bevy's ScreenshotPlugin rides in
    // DefaultPlugins. Mirrors the presenter's readback harness so it runs on a test thread.
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

    // An offscreen render target; COPY_SRC so the screenshot readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // A camera rendering into the offscreen image (a solid clear colour is enough for a PNG).
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

    // The crate's Screenshot import + save_to_disk observer, targeting the offscreen image.
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

/// Read whether the capture has been requested (the pipeline-wired signal).
fn progress_requested(app: &App) -> bool {
    app.world()
        .get_resource::<CaptureProgress>()
        .is_some_and(CaptureProgress::is_requested)
}

/// Whether the app's world holds at least one `Screenshot` entity (the `settle_then_capture` spawn).
fn spawned_a_screenshot(app: &mut App) -> bool {
    let mut query = app.world_mut().query::<&Screenshot>();
    query.iter(app.world()).next().is_some()
}
