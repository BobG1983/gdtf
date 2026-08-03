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

const TARGET_W: u32 = 800;

const TARGET_H: u32 = 600;

const MAX_CAPTURE_UPDATES: usize = 240;

const SETTLE_UPDATES: usize = 24;

const OUT_ENV: &str = "GDTF_OPTIONS_SHOT_OUT";

const OUT_ENV_OFF: &str = "GDTF_OPTIONS_SHOT_OUT_OFF";

fn resolve_out(env_var: &str, stem: &str) -> (std::path::PathBuf, bool) {
    match std::env::var(env_var) {
        Ok(path) if !path.is_empty() => (std::path::PathBuf::from(path), true),
        _ => (
            std::env::temp_dir().join(format!("{stem}_{}.png", std::process::id())),
            false,
        ),
    }
}

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
    app.set_error_handler(warn);
    app.add_plugins(UiPlugin);
    app.insert_resource(default_theme());
    app.insert_resource(GameSettings::default().with_sound(sound));

    let mut target =
        Image::new_target_texture(TARGET_W, TARGET_H, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.03)),
            ..default()
        },
        RenderTarget::Image(target_handle.clone().into()),
        IsDefaultUiCamera,
    ));

    let spawned = app.world_mut().run_system_once(spawn_options_screen);
    assert!(spawned.is_ok(), "spawn_options_screen must run");

    app.finish();
    app.cleanup();

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
