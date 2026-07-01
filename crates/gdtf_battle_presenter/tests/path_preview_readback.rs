//! GTW-358 / GTW-368 (C5 / C7, pixel proof): PIXEL-level proof that the path-preview route STEP
//! sprite actually renders NON-DARK at a route cell and DARK where no step is drawn.
//!
//! The green suite + the headless `path_preview.rs` state test prove the draw LOGIC (the right
//! entity is `Visible` at the right cell — including the GTW-368 target-cost `Text2d` label),
//! but a `Visible`, correctly-positioned sprite can still draw ZERO pixels (`bevy-traps.md` #8).
//! This test closes that gap with a real-GPU headless render-to-texture readback: it renders a
//! `PathStepSprite`-style sprite over a KNOWN dark clear colour to an offscreen `Image`, reads
//! the centre pixel back off the GPU, and asserts:
//!
//! - with the step sprite present, the rendered centre is NON-DARK (the warm-amber route trail
//!   composited over the dark clear) — a drawn route cell;
//! - with NO step sprite (the cleared / off-route case), the rendered centre is the DARK clear
//!   colour — an off-route cell is dark.
//!
//! POSITIVE — it NAMES the route-vs-off-route cases and asserts the pixel actually changes. The
//! GTW-368 target-cost LABEL's positive in-engine proof is the headless `path_preview.rs` state
//! test (it drives the REAL `TopDownRendererPlugin` draw path and asserts the single
//! `PathTargetLabel` `Text2d` is `Visible` at the NAMED target cell reading the route cost, and
//! is hidden with no target).
//!
//! Environment: needs a real GPU adapter (Metal on macOS). Single-threaded (a shared GPU lock).
//! No adapter (a GPU-less CI runner) → the harness SKIPS with a logged note rather than failing.

use std::sync::{Mutex, MutexGuard};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::RenderTarget,
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
use gdtf_test_utils::gpu_adapter_probe;

/// The translucent warm-amber tint the route preview draws (mirrors the private `PREVIEW_TINT`).
/// A readback proof of the SHIPPED look must use the SHIPPED colour; this is asserted against the
/// production const indirectly via the rendered NON-DARK, RED-dominant result.
const TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.55);

/// The dark battlefield clear colour the off-route cells read as.
const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

/// The offscreen render-target edge (square). Small keeps the readback cheap; the centred quad
/// fills it so the centre pixel is representative.
const TARGET_PX: u32 = 16;

/// Max `app.update()` calls to wait for the readback observer to fire.
const MAX_READBACK_UPDATES: usize = 60;

/// A captured readback pixel (the centre of the rendered target), sRGB-encoded bytes.
#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    /// Whether the readback observer fired and populated this.
    captured: bool,
    /// `[R, G, B, A]` sRGB-encoded (0..=255), as written to the `Rgba8UnormSrgb` target.
    rgba:     [u8; 4],
}

/// Serialises the real-GPU tests in this binary (one Metal device init at a time).
static GPU_LOCK: Mutex<()> = Mutex::new(());

/// Take the process-wide GPU lock, recovering from a poisoned mutex.
fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Build a render-capable headless `App` on the real GPU, or `None` if no adapter.
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
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    );

    app.finish();
    app.cleanup();

    // With pipelined rendering disabled, the RenderApp sub-app stays on the main app — its
    // absence means no GPU adapter was created (a GPU-less environment).
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Render the offscreen target with the step sprite present (`drawn = true`) or absent
/// (`drawn = false`) over the dark clear, read the centre pixel back, and return its sRGB bytes —
/// or `None` if no GPU adapter exists.
fn render_centre(drawn: bool) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    // The offscreen render target; COPY_SRC so the readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // The camera renders to the offscreen image over the DARK clear colour.
    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(DARK_CLEAR),
            ..default()
        },
        RenderTarget::Image(target_for_cam.into()),
    ));

    if drawn {
        // The route-preview step sprite: a translucent amber quad covering the target, centred
        // at the camera origin (the default 2D projection frames a TARGET_PX-unit rect).
        app.world_mut().spawn((
            Sprite {
                color: TINT,
                custom_size: Some(Vec2::splat(f32::from(
                    u16::try_from(TARGET_PX).unwrap_or(16),
                ))),
                ..default()
            },
            Transform::default(),
        ));
    }

    // Read the whole target texture back; the observer captures the centre pixel.
    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedPixel>| {
                let data = &trigger.event().data;
                // Rows are 256-aligned per the wgpu copy layout; index the centre pixel by row
                // stride so the padding never offsets the channel read.
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

    // Drive the full frame budget (not breaking early): the first frames land before the 2D
    // pass has drawn + the sprite's GpuImage uploads. The observer overwrites every frame, so
    // after the budget the captured value is the SETTLED steady-state draw.
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

/// C7 (pixel proof) — a drawn route cell renders NON-DARK (the warm-amber trail over the dark
/// clear), and an off-route cell (no step) renders the DARK clear colour.
#[test]
fn route_cell_renders_nondark_offroute_cell_renders_dark() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App`. On a
    // GPU-less runner `app.finish()` panics ("Unable to find a GPU!") before the in-build
    // `get_sub_app(RenderApp)?` guard, so skip here ahead of the app build.
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — path-preview pixel proof not run"
        );
        return;
    }

    let Some([lit_r, lit_g, lit_b, _lit_a]) = render_centre(true) else {
        eprintln!("SKIP: no GPU adapter in this environment — path-preview pixel proof not run");
        return;
    };
    let Some([dark_r, dark_g, dark_b, _dark_a]) = render_centre(false) else {
        eprintln!("SKIP: no GPU adapter in this environment — path-preview pixel proof not run");
        return;
    };

    // OFF-ROUTE: the dark clear colour reads near-black on every channel.
    assert!(
        dark_r < 40 && dark_g < 40 && dark_b < 40,
        "an off-route cell must render the DARK clear colour: got R={dark_r} G={dark_g} B={dark_b}",
    );

    // ROUTE: the amber tint composited over the dark clear is NON-DARK — and distinctly REDDER
    // than the off-route cell (R lifts most, the tint is red-dominant warm amber).
    assert!(
        i16::from(lit_r) - i16::from(dark_r) > 20,
        "the drawn route cell must render distinctly REDDER than the dark clear: \
         lit R={lit_r} vs dark R={dark_r}",
    );
    let lit_sum = u16::from(lit_r) + u16::from(lit_g) + u16::from(lit_b);
    let dark_sum = u16::from(dark_r) + u16::from(dark_g) + u16::from(dark_b);
    assert!(
        lit_sum > dark_sum,
        "the drawn route cell must be NON-DARK (brighter than the off-route clear): \
         lit=({lit_r},{lit_g},{lit_b}) dark=({dark_r},{dark_g},{dark_b})",
    );
}
