//! PIXEL-level proof that the [`TerrainFogMaterial`] WGSL fragment shader actually
//! desaturates (GTW-348 C9).
//!
//! The green suite does NOT validate WGSL — shaders load at runtime, so a shader
//! typo or a Rust-`ShaderType`-vs-WGSL layout mismatch passes `fmt`/`clippy`/`test`
//! yet breaks the render. This test closes that gap with a **real-GPU headless
//! render-to-texture readback**: it builds a render-capable `App` on the real Metal
//! backend (no window), renders a single [`TerrainFogMaterial`] quad over a KNOWN
//! solid-colour in-memory texture (NOT the shipped atlas) to an offscreen `Image`,
//! reads the rendered pixels back off the GPU via [`Readback`], and asserts the
//! desaturation contract on the actual shader output:
//!
//! - at `saturation = 0.0` the output is GREYSCALE (`R ~= G ~= B`) at the source's
//!   preserved BT.709 luminance (the EXPLORED / "was visible" memory cue);
//! - at `saturation = 1.0` the output RETAINS the source hue (full colour, the
//!   VISIBLE cell).
//!
//! Colour-space accounting: the test texture and the render target are both
//! `Rgba8UnormSrgb`. The GPU hardware linearises the sampled sRGB texture before the
//! fragment runs, the BT.709 luma + `mix` operate in linear light, and the target
//! re-encodes to sRGB on write — so the readback bytes are sRGB-encoded. The
//! greyscale assertion (`R == G == B`) is invariant under the sRGB round-trip
//! (channel-identical), and the luminance check decodes the readback grey back to
//! linear before comparing to the linear BT.709 luma of the linear source.
//!
//! Environment: this needs a real GPU adapter (Metal on macOS). It runs single
//! threaded — multiple simultaneous Bevy `App`s each spinning up a Metal device can
//! exhaust the device under parallel test load — and forces synchronous pipeline
//! compilation + disables pipelined rendering so the readback resolves within a
//! bounded `app.update()` loop on this thread. If no adapter is present (e.g. a
//! GPU-less CI runner) the harness skips with a logged note rather than failing —
//! the proof is valid where a GPU exists, which is the machine this fix renders on.

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
use gdtf_battle_presenter::TerrainFogMaterial;

/// The workspace-root `assets/` directory as an absolute path.
///
/// The custom-material shader loads from `assets/shaders/terrain_fog_material.wgsl`;
/// without pointing the `AssetServer` at the workspace `assets/` (the default is the
/// crate-local `crates/gdtf_battle_presenter/assets/`, which has no shaders) the WGSL
/// never loads, the pipeline never compiles, and the quad never draws — so this path
/// is load-bearing for the proof. Computed lexically from this crate's manifest dir
/// (`crates/gdtf_battle_presenter` → up two levels → `assets`), mirroring the running
/// app's asset root, the same way `GdtfLoadTestAppBuilder` resolves it.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The offscreen render-target edge (square). Small keeps the readback cheap; a
/// flat-coloured quad fills it uniformly so any interior pixel is representative.
const TARGET_PX: u32 = 16;

/// Max `app.update()` calls to wait for the readback observer to fire. With
/// synchronous pipeline compilation + pipelined rendering disabled the readback
/// resolves in a handful of frames; this is a generous cap before declaring a hang.
const MAX_READBACK_UPDATES: usize = 60;

/// A captured readback pixel (the centre of the rendered quad), sRGB-encoded bytes.
#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    /// Whether the readback observer fired and populated this.
    captured: bool,
    /// `[R, G, B, A]` sRGB-encoded (0..=255), as written to the `Rgba8UnormSrgb` target.
    rgba:     [u8; 4],
}

/// sRGB-encode a single linear channel (IEC 61966-2-1), to 0..=255.
fn srgb_encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055_f32.mul_add(c.powf(1.0 / 2.4), -0.055)
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// sRGB-decode a single 0..=255 channel to linear light (IEC 61966-2-1).
fn srgb_decode(byte: u8) -> f32 {
    let s = f32::from(byte) / 255.0;
    if s <= 0.040_45 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// BT.709 luma of a linear-light RGB triple — the exact weights the WGSL uses.
fn bt709_luma(linear_rgb: [f32; 3]) -> f32 {
    linear_rgb[0].mul_add(
        0.2126,
        linear_rgb[1].mul_add(0.7152, linear_rgb[2] * 0.0722),
    )
}

/// A solid 2×2 sRGB texture of one colour (`Rgba8UnormSrgb`).
fn solid_image(rgba: [u8; 4]) -> Image {
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
fn build_render_app() -> Option<App> {
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
static GPU_LOCK: Mutex<()> = Mutex::new(());

/// Take the process-wide GPU lock, recovering from a poisoned mutex (a panic in a
/// prior GPU test must not block the next from running and reporting its own result).
fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Render one [`TerrainFogMaterial`] quad over `source` at `saturation`, read the
/// rendered centre pixel back off the GPU, and return its sRGB bytes — or `None` if
/// no GPU adapter exists in this environment.
fn render_and_read(source: [u8; 4], saturation: f32) -> Option<[u8; 4]> {
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
    // shader will desaturate, at the saturation under test.
    let material = TerrainFogMaterial {
        image: image_handle,
        atlas_layout: None,
        atlas_index: 0,
        custom_size: Some(Vec2::splat(f32::from(
            u16::try_from(TARGET_PX).unwrap_or(16),
        ))),
        saturation,
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

/// at saturation 0.0 the rendered output is GREYSCALE (R~=G~=B) at the source's
/// preserved BT.709 luminance — the EXPLORED memory cue.
#[test]
fn explored_saturation_zero_renders_greyscale_at_preserved_luma() {
    // Pure red: a strongly-saturated colour so greyscale collapse is unambiguous.
    let source = [255_u8, 0, 0, 255];
    let Some([r, g, b, a]) = render_and_read(source, 0.0) else {
        eprintln!("SKIP: no GPU adapter in this environment — greyscale proof not run");
        return;
    };

    // The opaque source must survive the alpha mask (a >= 0.5 -> not discarded).
    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

    // GREYSCALE: channels equal within sRGB-quantisation noise.
    let eps = 3_i16;
    assert!(
        (i16::from(r) - i16::from(g)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (R vs G)"
    );
    assert!(
        (i16::from(g) - i16::from(b)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (G vs B)"
    );

    // LUMINANCE PRESERVED: decode the readback grey to linear, compare to the
    // linear BT.709 luma of the linear source. Both in linear light.
    let src_linear = [
        srgb_decode(source[0]),
        srgb_decode(source[1]),
        srgb_decode(source[2]),
    ];
    let expected_luma = bt709_luma(src_linear);
    let got_luma = srgb_decode(r);
    assert!(
        (got_luma - expected_luma).abs() < 0.03,
        "sat=0 luma not preserved: got {got_luma:.4} (R={r}) want ~{expected_luma:.4}"
    );

    // And it must NOT be the source hue (red would be R high, G/B ~0). A genuine
    // greyscale of red sits well below 255 in R and well above 0 in G/B.
    let src_grey = srgb_encode(expected_luma);
    assert!(
        (i16::from(r) - i16::from(src_grey)).abs() <= 3,
        "sat=0 grey byte: got R={r} want ~{src_grey}"
    );
    assert!(
        r < 200,
        "sat=0 still looks like saturated red (R={r}); shader did NOT desaturate"
    );
}

/// at saturation 1.0 the rendered output RETAINS the source hue — the VISIBLE cell.
#[test]
fn visible_saturation_one_retains_source_hue() {
    // A mixed colour so "retains hue" is a real per-channel match, not a coincidence.
    let source = [200_u8, 60, 30, 255];
    let Some([r, g, b, a]) = render_and_read(source, 1.0) else {
        eprintln!("SKIP: no GPU adapter in this environment — hue-retention proof not run");
        return;
    };

    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

    // IDENTITY: at sat=1 the mix is fully the sampled colour; output == source
    // within the sRGB sample/encode round-trip quantisation.
    let eps = 4_i16;
    assert!(
        (i16::from(r) - i16::from(source[0])).abs() <= eps,
        "sat=1 R not preserved: got {r} want ~{}",
        source[0]
    );
    assert!(
        (i16::from(g) - i16::from(source[1])).abs() <= eps,
        "sat=1 G not preserved: got {g} want ~{}",
        source[1]
    );
    assert!(
        (i16::from(b) - i16::from(source[2])).abs() <= eps,
        "sat=1 B not preserved: got {b} want ~{}",
        source[2]
    );

    // And it must NOT have collapsed to grey (the channels must differ — hue kept).
    assert!(
        (i16::from(r) - i16::from(b)).abs() > 20,
        "sat=1 collapsed toward grey: R={r} G={g} B={b}; shader over-desaturated"
    );
}
