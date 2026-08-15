use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, RenderAssetUsages},
    camera::RenderTarget,
    image::Image,
    math::Vec2,
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    sprite_render::Material2dPlugin,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{Brightness, TerrainFogMaterial};

use super::color::CapturedPixel;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) const TARGET_PX: u32 = 16;

/// Frames the hand-inserted scene needs to extract, prepare and draw — per-frame work, no IO.
const SCENE_SETTLE_FRAMES: u32 = 8;

pub(crate) fn solid_image(rgba: [u8; 4]) -> Image {
    let data: Vec<u8> = rgba.iter().copied().cycle().take(4 * 2 * 2).collect();
    Image::new(
        Extent3d {
            width:                 2,
            height:                2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

pub(crate) fn build_render_app() -> Option<App> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            })
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
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    )
    .add_plugins(Material2dPlugin::<TerrainFogMaterial>::default());

    app.finish();
    app.cleanup();

    app.get_sub_app(RenderApp)?;
    Some(app)
}

pub(crate) static GPU_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn render_and_read(
    source: [u8; 4],
    saturation: f32,
    brightness: Brightness,
) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    let image_handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(solid_image(source));

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    let material = TerrainFogMaterial {
        image: image_handle,
        atlas_layout: None,
        atlas_index: 0,
        custom_size: Some(Vec2::splat(f32::from(
            u16::try_from(TARGET_PX).unwrap_or(16),
        ))),
        saturation,
        brightness,
    };
    let mat_handle = app
        .world_mut()
        .resource_mut::<Assets<TerrainFogMaterial>>()
        .add(material);

    let mesh_handle = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(1.0, 1.0));

    // `Camera` `#[require]`s — not a `Camera` field. Point it at the offscreen image.
    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(1.0, 0.0, 1.0)),
            ..default()
        },
        RenderTarget::Image(target_for_cam.into()),
    ));
    app.world_mut().spawn((
        Mesh2d(mesh_handle),
        MeshMaterial2d(mat_handle),
        Transform::default(),
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
    Some(captured.rgba)
}
