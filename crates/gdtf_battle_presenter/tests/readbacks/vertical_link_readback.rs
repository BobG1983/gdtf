//! GPU readback: stair and ladder terrain tiles render nonempty and distinct.

use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    camera::RenderTarget,
    image::{Image, TextureAtlas, TextureAtlasLayout},
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_test_utils::gpu_adapter_probe;

const STAIR_UP_INDEX: usize = 29;

const LADDER_INDEX: usize = 235;

const TERRAIN_COLS: u32 = 16;
const TERRAIN_ROWS: u32 = 22;
const TERRAIN_TILE_PX: u32 = 16;

const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

const TARGET_PX: u32 = 16;

/// Frames the hand-inserted scene needs to extract, prepare and draw — per-frame work, no IO.
const SCENE_SETTLE_FRAMES: u32 = 8;

#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    captured:       bool,
    rgba:           [u8; 4],
    max_brightness: u16,
}

static GPU_LOCK: Mutex<()> = Mutex::new(());

fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
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
    );

    app.finish();
    app.cleanup();

    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn render_tile(index: Option<usize>) -> Option<([u8; 4], u16)> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

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
        RenderTarget::Image(target_for_cam.into()),
    ));

    if let Some(index) = index {
        let image: Handle<Image> = app
            .world()
            .resource::<AssetServer>()
            .load("sprites/alt_tileset_terrain.png");
        let layout = TextureAtlasLayout::from_grid(
            UVec2::splat(TERRAIN_TILE_PX),
            TERRAIN_COLS,
            TERRAIN_ROWS,
            None,
            None,
        );
        let layout_handle = app
            .world_mut()
            .resource_mut::<Assets<TextureAtlasLayout>>()
            .add(layout);
        let mut sprite = Sprite::from_atlas_image(
            image,
            TextureAtlas {
                layout: layout_handle,
                index,
            },
        );
        sprite.custom_size = Some(Vec2::splat(f32::from(
            u16::try_from(TARGET_PX).unwrap_or(16),
        )));
        app.world_mut().spawn((sprite, Transform::default()));
    }

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
                let edge = TARGET_PX as usize;
                let mut sums = [0u32; 4];
                let mut count = 0u32;
                let mut max_brightness = 0u16;
                for cy in 0..edge {
                    for cx in 0..edge {
                        let off = cy * row_stride + cx * 4;
                        if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
                            data.get(off),
                            data.get(off + 1),
                            data.get(off + 2),
                            data.get(off + 3),
                        ) {
                            sums[0] += u32::from(r);
                            sums[1] += u32::from(g);
                            sums[2] += u32::from(b);
                            sums[3] += u32::from(a);
                            count += 1;
                            let texel = u16::from(r) + u16::from(g) + u16::from(b);
                            max_brightness = max_brightness.max(texel);
                        }
                    }
                }
                let mean = |sum: u32| {
                    sum.checked_div(count)
                        .and_then(|m| u8::try_from(m).ok())
                        .unwrap_or(0)
                };
                if count > 0 {
                    captured.captured = true;
                    captured.rgba = [mean(sums[0]), mean(sums[1]), mean(sums[2]), mean(sums[3])];
                    captured.max_brightness = max_brightness;
                }
            },
        );

    while !app.world().resource::<CapturedPixel>().captured {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    Some((captured.rgba, captured.max_brightness))
}

#[test]
fn stair_and_ladder_render_nonempty_and_distinct() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — vertical-link pixel proof not run"
        );
        return;
    }

    let Some((baseline, base_max)) = render_tile(None) else {
        eprintln!("SKIP: no GPU adapter in this environment — vertical-link pixel proof not run");
        return;
    };
    let Some((stair, stair_max)) = render_tile(Some(STAIR_UP_INDEX)) else {
        eprintln!("SKIP: no GPU adapter in this environment — vertical-link pixel proof not run");
        return;
    };
    let Some((ladder, ladder_max)) = render_tile(Some(LADDER_INDEX)) else {
        eprintln!("SKIP: no GPU adapter in this environment — vertical-link pixel proof not run");
        return;
    };

    assert!(
        baseline[0] < 40 && baseline[1] < 40 && baseline[2] < 40,
        "the no-link baseline must render the DARK clear colour: got {baseline:?}",
    );

    assert!(
        stair_max > base_max + 60,
        "the stair-up tile (index 29) must render NON-EMPTY (a lit texel far above the dark \
         baseline): stair max {stair_max} vs baseline max {base_max}",
    );
    assert!(
        ladder_max > base_max + 60,
        "the ladder tile (index 235) must render NON-EMPTY (a lit texel far above the dark \
         baseline): ladder max {ladder_max} vs baseline max {base_max}",
    );

    let channel_diff = (i16::from(stair[0]) - i16::from(ladder[0])).abs()
        + (i16::from(stair[1]) - i16::from(ladder[1])).abs()
        + (i16::from(stair[2]) - i16::from(ladder[2])).abs();
    assert!(
        channel_diff > 5,
        "the stair-up (index 29) and ladder (index 235) tiles must render DISTINCT from \
         each other: stair mean={stair:?} vs ladder mean={ladder:?} (channel diff {channel_diff})",
    );
}
