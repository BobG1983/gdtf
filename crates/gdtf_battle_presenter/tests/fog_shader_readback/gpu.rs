//! The real-GPU offscreen render + `Readback` pipeline (serialised by the GPU lock).

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

/// The workspace-root `assets/` directory as an absolute path.
///
/// The custom-material shader loads from `assets/shaders/terrain_fog_material.wgsl`;
/// without pointing the `AssetServer` at the workspace `assets/` (the default is the
/// crate-local `crates/gdtf_battle_presenter/assets/`, which has no shaders) the WGSL
/// never loads, the pipeline never compiles, and the quad never draws — so this path
/// is load-bearing for the proof. Computed lexically from this crate's manifest dir
/// (`crates/gdtf_battle_presenter` → up two levels → `assets`), mirroring the running
/// app's asset root, the same way `GdtfLoadTestAppBuilder` resolves it.
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The offscreen render-target edge (square). Small keeps the readback cheap; a
/// flat-coloured quad fills it uniformly so any interior pixel is representative.
pub(crate) const TARGET_PX: u32 = 16;

/// Max `app.update()` calls to wait for the readback observer to fire. With
/// synchronous pipeline compilation + pipelined rendering disabled the readback
/// resolves in a handful of frames; this is a generous cap before declaring a hang.
pub(crate) const MAX_READBACK_UPDATES: usize = 60;

/// A solid 2×2 sRGB texture of one colour (`Rgba8UnormSrgb`).
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

/// Build a render-capable headless `App` on the real GPU, or `None` if no adapter.
pub(crate) fn build_render_app() -> Option<App> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                // Point at the workspace `assets/` so the custom-material WGSL loads
                // (without this the shader is "Path not found" and the quad never draws).
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                // Block until the WGSL -> MSL pipeline compiles before the first
                // draw, so the readback isn't read off a not-yet-rendered target.
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>()
            // Keep the render world synchronous within app.update() so the readback
            // observer fires on this thread within the bounded loop.
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    )
    // `GpuReadbackPlugin` already rides in `DefaultPlugins` in 0.19; only the
    // material plugin must be added.
    .add_plugins(Material2dPlugin::<TerrainFogMaterial>::default());

    app.finish();
    app.cleanup();

    // With pipelined rendering disabled, the RenderApp sub-app stays on the main
    // app — its absence means no GPU adapter was created (GPU-less environment).
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Serialises the real-GPU tests in this binary. The default test harness runs
/// tests in parallel; two Bevy `App`s each spinning up a Metal device at once can
/// exhaust the device under parallel load (per the GTW-348 PROVE recon), so each
/// GPU render holds this lock for its lifetime — turning the two tests into a
/// sequence without forcing `--test-threads=1` on the whole suite.
pub(crate) static GPU_LOCK: Mutex<()> = Mutex::new(());

/// Take the process-wide GPU lock, recovering from a poisoned mutex (a panic in a
/// prior GPU test must not block the next from running and reporting its own result).
pub(crate) fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Render one [`TerrainFogMaterial`] quad over `source` at `saturation` + `brightness`, read
/// the rendered centre pixel back off the GPU, and return its sRGB bytes — or `None` if no GPU
/// adapter exists in this environment. `brightness` is the GTW-519 storey-depth scalar the
/// shader multiplies the (grey-mixed) RGB by AFTER the saturation mix.
pub(crate) fn render_and_read(
    source: [u8; 4],
    saturation: f32,
    brightness: Brightness,
) -> Option<[u8; 4]> {
    // Held for the whole GPU lifetime so the two GPU tests never init Metal at once.
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    // The known solid-colour source texture (NOT the shipped atlas).
    let image_handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(solid_image(source));

    // The offscreen render target; add COPY_SRC so the readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // The fog material: identity UV (no atlas layout = whole image), the colour the
    // shader will desaturate, at the saturation + brightness under test.
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

    // A unit-rect mesh scaled by the material's vertex_scale (custom_size) in the
    // vertex stage, so the quad fully covers the target.
    let mesh_handle = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(1.0, 1.0));

    // In Bevy 0.19 the render target is a separate `RenderTarget` component the
    // `Camera` `#[require]`s — not a `Camera` field. Point it at the offscreen image.
    // A distinctive magenta clear colour makes a "quad never drew" failure obvious:
    // a clear-only frame reads magenta, never the desaturated tile, so a regression
    // that stops the quad rendering can't masquerade as a pass.
    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(1.0, 0.0, 1.0)),
            ..default()
        },
        RenderTarget::Image(target_for_cam.into()),
    ));
    // The quad is a UNIT rect; the shader's vertex stage scales it by the material's
    // `vertex_scale` (the `custom_size` of `TARGET_PX`). Under the default 2D
    // `WindowSize` projection a `TARGET_PX`-sized target frames a `TARGET_PX`-unit
    // world rect, so the scaled quad fully covers the target and the centre pixel
    // samples the tile colour.
    app.world_mut().spawn((
        Mesh2d(mesh_handle),
        MeshMaterial2d(mat_handle),
        Transform::default(),
    ));

    // Read the whole target texture back; the observer captures the centre pixel.
    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedPixel>| {
                let data = &trigger.event().data;
                // Rows are aligned to 256 bytes per the wgpu copy layout; index the
                // centre pixel by row stride so padding never offsets the channel read.
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

    // Drive the full frame budget, NOT breaking on the first readback. The early
    // readbacks land on frames before the camera's 2D pass has drawn the quad (the
    // readback copy can run ahead of the first steady-state draw) and before the
    // custom-shader pipeline compiles + the source image's GpuImage uploads (until
    // then `AsBindGroupShaderType` returns the zero-`vertex_scale` `Default`, which
    // collapses the quad). The observer overwrites `rgba` every frame, so after the
    // budget the captured value is the SETTLED steady-state tile draw.
    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates"
    );
    Some(captured.rgba)
}
