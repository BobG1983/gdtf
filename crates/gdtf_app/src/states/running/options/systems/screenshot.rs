//! GPU-guarded screenshot capture of the RENDERED Options screen (GTW-637, TESTING
//! clause: "one screenshot via the existing capture harness").
//!
//! The headless entity-assert tests (`super::test`, `tests/options_scene.rs`) prove the
//! world effects of the real systems on a `MinimalPlugins` harness, but they cannot
//! rasterize the screen — there is no GPU under `MinimalPlugins`. This test closes that
//! gap: it builds a render-capable, windowless app, adds the real
//! [`UiPlugin`](gdtf_ui::UiPlugin) + theme, spawns the REAL `bsn!` + `gdtf_ui`-widget
//! screen via [`spawn_options_screen`], renders it into an offscreen target, and writes a
//! PNG through the SAME `Screenshot::image` + `save_to_disk` pipeline the
//! `gdtf_screenshot` crate wraps (its documented core; see
//! `gdtf_screenshot/tests/capture_pipeline.rs::real_gpu_capture_writes_a_png`). The PNG is
//! the agent-readable visual-QA artifact a QA pass can `Read` to assert the layout.
//!
//! GPU-guarded per GTW-527: on a GPU-less runner it logs a skip and returns BEFORE
//! building the render app, so it never panics inside `app.finish()`. When the
//! `GDTF_OPTIONS_SHOT_OUT` env var is set, the capture is written there and KEPT (the QA /
//! in-transcript evidence path); otherwise it goes to a unique temp file and is removed on
//! success (the version-controlled tree stays clean, the GTW-555 isolation precedent).

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    camera::RenderTarget,
    ecs::{error::warn, system::RunSystemOnce},
    image::Image,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{TextureFormat, TextureUsages},
        view::window::screenshot::{Screenshot, save_to_disk},
    },
    ui::IsDefaultUiCamera,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_test_utils::gpu_probe::{GpuAdapterProbe, gpu_adapter_probe};
use gdtf_ui::{UiPlugin, theme::default_theme};

use super::spawn_options_screen;
use crate::states::running::options::settings::GameSettings;

/// The offscreen target's width in pixels — a landscape window shape so the centered
/// panel + Continue button lay out as they do in-game.
const TARGET_W: u32 = 800;

/// The offscreen target's height in pixels.
const TARGET_H: u32 = 600;

/// Max `app.update()` calls to wait for the async GPU readback to flush the PNG to disk.
const MAX_CAPTURE_UPDATES: usize = 240;

/// Frames to let the UI lay out + render before the capture is spawned.
const SETTLE_UPDATES: usize = 24;

/// The opt-in env var that pins the output path and KEEPS the PNG (QA / evidence path).
const OUT_ENV: &str = "GDTF_OPTIONS_SHOT_OUT";

/// Renders the real Options screen to an offscreen target and asserts a PNG lands on disk.
///
/// The proof the TESTING clause's "one screenshot via the existing capture harness"
/// requires: the actual rendered frame of the pilot screen (themed panel, the `gdtf_ui`
/// sound switch, the Continue button), captured through Bevy's `Screenshot` render path.
#[test]
fn renders_the_options_screen_to_a_png() {
    if gpu_adapter_probe() == GpuAdapterProbe::Absent {
        eprintln!(
            "SKIP renders_the_options_screen_to_a_png: no usable wgpu adapter (GPU-less \
             runner) — the screen's structure/theming are covered by the headless tests."
        );
        return;
    }

    let (out, keep) = match std::env::var(OUT_ENV) {
        Ok(path) if !path.is_empty() => (std::path::PathBuf::from(path), true),
        _ => (
            std::env::temp_dir().join(format!("gdtf_options_screen_{}.png", std::process::id())),
            false,
        ),
    };
    drop(std::fs::remove_file(&out));

    // A render-capable headless app (no window, no winit): real wgpu device via
    // DefaultPlugins, driven on the test thread (the gdtf_screenshot / present-path recipe).
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
    // A failed render-provided param validation logs + skips rather than panicking (the
    // shared DefaultPlugins-headless harness precedent).
    app.set_error_handler(warn);
    // The real theming pass (paints the Themed backdrop/title/text nodes) + widget drivers.
    app.add_plugins(UiPlugin);
    app.insert_resource(default_theme());
    app.init_resource::<GameSettings>();

    // An offscreen render target the UI camera renders into; COPY_SRC so the screenshot
    // readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_W, TARGET_H, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // The sole camera, marked the default UI camera so bevy_ui binds the screen's roots to
    // it — without a primary window there is no implicit default, so UI would otherwise
    // render nowhere (the GTW-764 present-path lesson; see `net_qa::present::retarget`).
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.03)),
            ..default()
        },
        RenderTarget::Image(target_handle.clone().into()),
        IsDefaultUiCamera,
    ));

    // Spawn the REAL Options screen (the same system OnEnter(Options) runs).
    let spawned = app.world_mut().run_system_once(spawn_options_screen);
    assert!(spawned.is_ok(), "spawn_options_screen must run");

    app.finish();
    app.cleanup();

    // Let the UI lay out + render a few frames before the capture is taken.
    for _ in 0..SETTLE_UPDATES {
        app.update();
    }

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
        "the Options screen capture must write a PNG to {}",
        out.display()
    );
    if keep {
        eprintln!("Options screen screenshot kept at {}", out.display());
    } else {
        drop(std::fs::remove_file(&out));
    }
}
