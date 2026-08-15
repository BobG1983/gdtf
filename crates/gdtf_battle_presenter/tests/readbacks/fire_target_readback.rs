//! Fire target GPU readback: highlighted cell renders red; cleared cell is dark.
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::error::warn,
    image::Image,
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    FireTargetHighlight, FireTargetTile, TopDownRendererPlugin, WORLD_RENDER_LAYER,
};
use gdtf_battle_sim::prelude::{BattleInProgress, Cell, CellLevel, Level, Tu};
use gdtf_test_utils::gpu_adapter_probe;

const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

const TARGET_PX: u32 = 16;

/// Frames the hand-inserted scene needs to extract, prepare and draw — per-frame work, no IO.
const SCENE_SETTLE_FRAMES: u32 = 8;

#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    captured: bool,
    rgba:     [u8; 4],
}

static GPU_LOCK: Mutex<()> = Mutex::new(());

fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn build_render_app() -> Option<App> {
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
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    app.set_error_handler(warn);

    app.finish();
    app.cleanup();

    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn render_centre(drawn: bool) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    let cell = CellLevel::new(Cell::new(0, 0), Level::new(0));
    if drawn {
        app.world_mut()
            .insert_resource(FireTargetHighlight::new(cell, Tu::new(14)));
    } else {
        app.world_mut()
            .insert_resource(FireTargetHighlight::cleared());
    }

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(DARK_CLEAR),
            ..default()
        },
        Transform::default(),
        RenderTarget::Image(target_for_cam.into()),
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));

    // Let the scene draw before any copy is submitted, so every readback is of a settled frame.
    for _ in 0..SCENE_SETTLE_FRAMES {
        app.update();
    }

    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedPixel>| {
                let data = &trigger.event().data;
                let row_stride = data.len() / TARGET_PX as usize;
                let cx = (TARGET_PX / 2) as usize;
                let cy = (TARGET_PX / 2) as usize;
                let off = cy * row_stride + cx * 4;
                if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
                    data.get(off),
                    data.get(off + 1),
                    data.get(off + 2),
                    data.get(off + 3),
                ) {
                    captured.captured = true;
                    captured.rgba = [r, g, b, a];
                }
            },
        );

    while !app.world().resource::<CapturedPixel>().captured {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    if drawn {
        let mut q = app.world_mut().query::<&FireTargetTile>();
        assert!(
            q.iter(app.world()).next().is_some(),
            "the REAL draw_fire_target must have spawned the pooled FireTargetTile",
        );
    }
    Some(captured.rgba)
}

#[test]
fn fire_target_cell_renders_red_cleared_cell_renders_dark() {
    // GPU-less runner (no adapter) `app.finish()` would `.expect("Unable to find a GPU!")`
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — fire-target pixel proof not run"
        );
        return;
    }

    let Some([lit_r, lit_g, lit_b, _lit_a]) = render_centre(true) else {
        eprintln!("SKIP: no GPU adapter in this environment — fire-target pixel proof not run");
        return;
    };
    let Some([dark_r, dark_g, dark_b, _dark_a]) = render_centre(false) else {
        eprintln!("SKIP: no GPU adapter in this environment — fire-target pixel proof not run");
        return;
    };

    assert!(
        dark_r < 40 && dark_g < 40 && dark_b < 40,
        "a cleared (non-target) cell must render the DARK clear colour: \
         got R={dark_r} G={dark_g} B={dark_b}",
    );

    assert!(
        i16::from(lit_r) - i16::from(lit_g) > 30 && i16::from(lit_r) - i16::from(lit_b) > 30,
        "the fire-target cell must render RED-DOMINANT (R clearly exceeds G and B): \
         got R={lit_r} G={lit_g} B={lit_b}",
    );
    assert!(
        i16::from(lit_r) - i16::from(dark_r) > 20,
        "the fire-target cell must render distinctly REDDER (non-dark) than the cleared clear: \
         lit R={lit_r} vs dark R={dark_r}",
    );
    let lit_sum = u16::from(lit_r) + u16::from(lit_g) + u16::from(lit_b);
    let dark_sum = u16::from(dark_r) + u16::from(dark_g) + u16::from(dark_b);
    assert!(
        lit_sum > dark_sum,
        "the fire-target cell must be NON-DARK (brighter than the cleared clear): \
         lit=({lit_r},{lit_g},{lit_b}) dark=({dark_r},{dark_g},{dark_b})",
    );
}
