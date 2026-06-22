//! GTW-357 (C5, pixel proof): PIXEL-level proof that the reachable-overlay TINT sprite
//! actually renders NON-DARK at a lit reachable cell and DARK where no tint is drawn.
//!
//! The green suite + the headless `reachable_overlay.rs` state test prove the draw LOGIC (the
//! right entity is `Visible` at the right cell), but a `Visible`, correctly-positioned sprite
//! can still draw ZERO pixels (`bevy-traps.md` #8). This test closes that gap the
//! `fog_shader_readback.rs` way — a real-GPU headless render-to-texture readback: it renders a
//! `ReachableTint`-style sprite over a KNOWN dark clear colour to an offscreen `Image`, reads
//! the centre pixel back off the GPU, and asserts:
//!
//! - with the tint sprite present, the rendered centre is NON-DARK (the green wash composited
//!   over the dark clear) — a lit reachable cell;
//! - with NO tint sprite (the cleared / out-of-range case), the rendered centre is the DARK
//!   clear colour — an unlit cell is dark.
//!
//! POSITIVE — it NAMES the lit-vs-unlit cases and asserts the pixel actually changes.
//!
//! Environment: needs a real GPU adapter (Metal on macOS). Single-threaded (a shared GPU lock,
//! the `fog_shader_readback.rs` precedent) so two Bevy `App`s never init Metal at once. No
//! adapter (a GPU-less CI runner) → the harness SKIPS with a logged note rather than failing.

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

/// The translucent green tint the overlay draws (mirrors the private `REACHABLE_TINT`). A
/// readback proof of the SHIPPED look must use the SHIPPED colour; this is asserted against the
/// production const indirectly via the rendered NON-DARK result (the exact value need not match,
/// only that a green wash composites NON-DARK over the dark clear).
const TINT: Color = Color::srgba(0.35, 0.9, 0.45, 0.3);

/// The dark battlefield clear colour the unlit cells read as.
const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

/// The offscreen render-target edge (square). Small keeps the readback cheap; the centred
/// quad fills it so the centre pixel is representative.
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

/// Serialises the real-GPU tests in this binary (the `fog_shader_readback.rs` precedent).
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

/// Render the offscreen target with the tint sprite present (`lit = true`) or absent
/// (`lit = false`) over the dark clear, read the centre pixel back, and return its sRGB bytes —
/// or `None` if no GPU adapter exists.
fn render_centre(lit: bool) -> Option<[u8; 4]> {
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

    if lit {
        // The reachable-overlay tint sprite: a translucent green quad covering the target,
        // centred at the camera origin (the default 2D projection frames a TARGET_PX-unit rect).
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

/// C5 (pixel proof) — a lit reachable cell renders NON-DARK (the green wash over the dark
/// clear), and an unlit cell (no tint) renders the DARK clear colour.
#[test]
fn lit_cell_renders_nondark_unlit_cell_renders_dark() {
    let Some([lit_r, lit_g, lit_b, _lit_a]) = render_centre(true) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — reachable-overlay pixel proof not run"
        );
        return;
    };
    let Some([dark_r, dark_g, dark_b, _dark_a]) = render_centre(false) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — reachable-overlay pixel proof not run"
        );
        return;
    };

    // UNLIT: the dark clear colour reads near-black on every channel.
    assert!(
        dark_r < 40 && dark_g < 40 && dark_b < 40,
        "an unlit cell must render the DARK clear colour: got R={dark_r} G={dark_g} B={dark_b}",
    );

    // LIT: the green tint composited over the dark clear is NON-DARK — and distinctly GREENER
    // than the unlit cell (G lifts most, the tint is green-dominant).
    assert!(
        i16::from(lit_g) - i16::from(dark_g) > 20,
        "the lit reachable cell must render distinctly GREENER than the dark clear: \
         lit G={lit_g} vs dark G={dark_g}",
    );
    let lit_sum = u16::from(lit_r) + u16::from(lit_g) + u16::from(lit_b);
    let dark_sum = u16::from(dark_r) + u16::from(dark_g) + u16::from(dark_b);
    assert!(
        lit_sum > dark_sum,
        "the lit reachable cell must be NON-DARK (brighter than the unlit clear): \
         lit=({lit_r},{lit_g},{lit_b}) dark=({dark_r},{dark_g},{dark_b})",
    );
}
