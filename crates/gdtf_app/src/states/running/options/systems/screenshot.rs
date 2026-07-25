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
//! Two states are captured, each behind its own opt-in env var: the default sound-ON
//! screen ([`OUT_ENV`]) and the sound-OFF screen ([`OUT_ENV_OFF`]). The OFF capture is
//! the visual proof of the GTW-800b fix — the OFF toggle track's opaque pill outline is
//! visible against the panel — and, read beside the ON capture, of GTW-800a: the panel
//! keeps the same bounds because the "On" / "Off" readout occupies a fixed width.
//!
//! GPU-guarded per GTW-527: on a GPU-less runner it logs a skip and returns BEFORE
//! building the render app, so it never panics inside `app.finish()`. When the capture's
//! env var is set, the PNG is written there and KEPT (the QA / in-transcript evidence
//! path); otherwise it goes to a unique temp file and is removed on success (the
//! version-controlled tree stays clean, the GTW-555 isolation precedent).

use std::path::Path;

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
use crate::states::running::options::settings::{GameSettings, SoundEnabled};

/// The offscreen target's width in pixels — a landscape window shape so the centered
/// panel + Continue button lay out as they do in-game.
const TARGET_W: u32 = 800;

/// The offscreen target's height in pixels.
const TARGET_H: u32 = 600;

/// Max `app.update()` calls to wait for the async GPU readback to flush the PNG to disk.
const MAX_CAPTURE_UPDATES: usize = 240;

/// Frames to let the UI lay out + render before the capture is spawned.
const SETTLE_UPDATES: usize = 24;

/// The opt-in env var that pins the sound-ON capture path and KEEPS the PNG (QA path).
const OUT_ENV: &str = "GDTF_OPTIONS_SHOT_OUT";

/// The opt-in env var that pins the sound-OFF capture path and KEEPS the PNG (QA path).
const OUT_ENV_OFF: &str = "GDTF_OPTIONS_SHOT_OUT_OFF";

/// Resolves an output path + keep flag from `env_var`: a set, non-empty value pins the
/// path and KEEPS the PNG (the QA / evidence path); otherwise a unique temp file that is
/// removed on success (the tree stays clean).
fn resolve_out(env_var: &str, stem: &str) -> (std::path::PathBuf, bool) {
    match std::env::var(env_var) {
        Ok(path) if !path.is_empty() => (std::path::PathBuf::from(path), true),
        _ => (
            std::env::temp_dir().join(format!("{stem}_{}.png", std::process::id())),
            false,
        ),
    }
}

/// Renders the REAL Options screen seeded with `sound` to an offscreen target and returns
/// whether a PNG landed at `out`.
///
/// Builds a render-capable headless app (no window, no winit): a real wgpu device via
/// `DefaultPlugins`, the real [`UiPlugin`](gdtf_ui::UiPlugin) theming pass + widget
/// drivers, and the persisted [`GameSettings`] seeded to `sound` so the toggle spawns in
/// the requested on/off state. The screen is spawned by the SAME
/// [`spawn_options_screen`] the running app runs `OnEnter(Options)`, rendered into an
/// offscreen image, and read back through Bevy's `Screenshot` path.
fn render_options_to_png(sound: SoundEnabled, out: &Path) -> bool {
    drop(std::fs::remove_file(out));

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
    app.insert_resource(GameSettings::default().with_sound(sound));

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

    let path = out.to_path_buf();
    app.world_mut()
        .spawn(Screenshot::image(target_handle))
        .observe(save_to_disk(path));

    for _ in 0..MAX_CAPTURE_UPDATES {
        app.update();
        if out.exists() {
            return true;
        }
    }
    false
}

/// Renders the default (sound-ON) Options screen and asserts a PNG lands on disk.
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
    let (out, keep) = resolve_out(OUT_ENV, "gdtf_options_screen");
    let written = render_options_to_png(SoundEnabled::new(true), &out);
    assert!(
        written,
        "the Options screen capture must write a PNG to {}",
        out.display()
    );
    if keep {
        eprintln!(
            "Options screen (sound ON) screenshot kept at {}",
            out.display()
        );
    } else {
        drop(std::fs::remove_file(&out));
    }
}

/// Renders the sound-OFF Options screen and asserts a PNG lands on disk (GTW-800a/b).
///
/// The visual-QA artifact for the two layout fixes: with sound OFF the toggle track's
/// opaque themed pill outline must be visible against the panel (GTW-800b), and — read
/// beside [`renders_the_options_screen_to_a_png`]'s ON capture — the panel keeps the
/// same bounds because the "Off" readout occupies the same fixed width as "On"
/// (GTW-800a). Set `GDTF_OPTIONS_SHOT_OUT_OFF` to keep the PNG for inspection.
#[test]
fn renders_the_sound_off_options_screen_to_a_png() {
    if gpu_adapter_probe() == GpuAdapterProbe::Absent {
        eprintln!(
            "SKIP renders_the_sound_off_options_screen_to_a_png: no usable wgpu adapter \
             (GPU-less runner) — the OFF toggle border is covered by the headless test \
             off_sound_toggle_track_has_a_visible_themed_border."
        );
        return;
    }
    let (out, keep) = resolve_out(OUT_ENV_OFF, "gdtf_options_screen_off");
    let written = render_options_to_png(SoundEnabled::new(false), &out);
    assert!(
        written,
        "the sound-OFF Options screen capture must write a PNG to {}",
        out.display()
    );
    if keep {
        eprintln!(
            "Options screen (sound OFF) screenshot kept at {}",
            out.display()
        );
    } else {
        drop(std::fs::remove_file(&out));
    }
}
