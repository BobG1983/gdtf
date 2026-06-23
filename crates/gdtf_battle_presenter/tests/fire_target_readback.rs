//! GTW-371 (C2 / C5, pixel proof): PIXEL-level proof that the fire-target RED tile actually
//! renders RED-DOMINANT at the hovered enemy cell and DARK where no fire target is drawn.
//!
//! The green suite + the headless `fire_target.rs` state test prove the draw LOGIC (the single
//! `FireTargetTile` sprite is `Visible` at the right cell, UNDER the actor band, with the OPAQUE
//! cost label), but a `Visible`, correctly-positioned sprite can still draw ZERO pixels
//! (`bevy-traps.md` #8). This test closes that gap with a real-GPU headless render-to-texture
//! readback that drives the REAL `draw_fire_target` system (via the production
//! `TopDownRendererPlugin`, the SAME wiring `fire_target.rs` uses) — but on a real Metal adapter
//! instead of `backends: None`. It sets the presenter-owned `FireTargetHighlight` to a NAMED cell
//! and fire cost, lets the real draw spawn and position the pooled red `Sprite`, renders the cell
//! centre to an offscreen `Image` over a KNOWN dark clear colour, reads the centre pixel back off
//! the GPU, and asserts:
//!
//! - with a fire-target highlight on the NAMED cell, the rendered centre is RED-DOMINANT (the R
//!   channel clearly exceeds G and B — the `FIRE_TARGET_TINT` red composited over the dark clear)
//!   and NON-DARK (brighter than the off-target clear) — a drawn fire-target cell;
//! - with a CLEARED highlight (no fireable hover), the same cell renders the DARK clear colour —
//!   a non-target cell is dark.
//!
//! POSITIVE — it NAMES the target-vs-cleared cases and asserts the pixel is actually red. It
//! exercises the SHIPPED draw (the real `FireTargetTile` sprite at the real `FIRE_TARGET_TINT`),
//! not a hand-rolled stand-in, so the rendered red is the shipped look.
//!
//! Environment: needs a real GPU adapter (Metal on macOS). Single-threaded (a shared GPU lock).
//! No adapter (a GPU-less CI runner) -> the harness SKIPS with a logged note rather than failing.

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
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level, Tu};

/// The dark battlefield clear colour the non-target cells read as.
const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

/// The offscreen render-target edge (square). Small keeps the readback cheap; the centred red
/// tile fills it so the centre pixel is representative of the fire-target tile.
const TARGET_PX: u32 = 16;

/// Max `app.update()` calls to wait for the real draw to spawn + position the pooled tile and the
/// readback observer to fire.
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

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets), so the
/// real `TopDownRendererPlugin`'s `Startup` atlas loads resolve.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Build a render-capable headless `App` on the real GPU wired with the production
/// `TopDownRendererPlugin` (so the REAL `draw_fire_target` runs), or `None` if no adapter.
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
    // The fire-target draw is `BattleInProgress`-gated; open the gate (the `fire_target.rs`
    // headless precedent) so the REAL `draw_fire_target` runs.
    app.insert_resource(BattleInProgress);
    // Warn-not-panic on a transient missing-resource gate race in the OTHER battle-gated draw
    // systems (this focused harness opens `BattleInProgress` without the full `setup_battle`, the
    // `fire_target.rs` precedent). The fire-target draw reads only the `init_resource`-d
    // `FireTargetHighlight` + `ActiveLevel`, so it always runs.
    app.set_error_handler(warn);

    app.finish();
    app.cleanup();

    // With pipelined rendering disabled, the RenderApp sub-app stays on the main app — its
    // absence means no GPU adapter was created (a GPU-less environment).
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Drive the REAL `draw_fire_target` with a fire-target highlight present (`drawn = true`) on the
/// NAMED cell, or cleared (`drawn = false`), render the offscreen target over the dark clear,
/// read the centre pixel back, and return its sRGB bytes — or `None` if no GPU adapter exists.
///
/// The highlight cell is `(0, 0, L0)`, which `cell_to_world` maps to the world ORIGIN — so the
/// offscreen camera (framed at the origin) renders that cell's red tile to the target centre.
fn render_centre(drawn: bool) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    // The NAMED fire-target cell at the world origin (so it lands at the camera centre).
    let cell = CellLevel::new(Cell::new(0, 0), Level::new(0));
    if drawn {
        app.world_mut()
            .insert_resource(FireTargetHighlight::new(cell, Tu::new(14)));
    } else {
        app.world_mut()
            .insert_resource(FireTargetHighlight::cleared());
    }

    // The offscreen render target; COPY_SRC so the readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // An offscreen camera framed at the world origin (the fire-target cell), rendering ONLY the
    // world render layer (where the fire-target tile draws) over the DARK clear colour. Centred at
    // origin so cell (0,0,L0) is at the target centre.
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

    // Drive the full frame budget (not breaking early): the first frames land before the real
    // draw's lazy pool spawn + command flush, before the 2D pass has drawn, and before the
    // sprite's GpuImage uploads. The observer overwrites every frame, so after the budget the
    // captured value is the SETTLED steady-state draw (settle-before-read).
    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates"
    );
    // Sanity: the real draw actually spawned the pooled tile when a highlight was present.
    if drawn {
        let mut q = app.world_mut().query::<&FireTargetTile>();
        assert!(
            q.iter(app.world()).next().is_some(),
            "the REAL draw_fire_target must have spawned the pooled FireTargetTile",
        );
    }
    Some(captured.rgba)
}

/// C5 (pixel proof) — the fire-target cell renders RED-DOMINANT + NON-DARK (the `FIRE_TARGET_TINT`
/// red tile over the dark clear) when a highlight is present, and the DARK clear colour when the
/// highlight is cleared. Drives the REAL `draw_fire_target` on a real GPU and reads the centre
/// pixel back.
#[test]
fn fire_target_cell_renders_red_cleared_cell_renders_dark() {
    let Some([lit_r, lit_g, lit_b, _lit_a]) = render_centre(true) else {
        eprintln!("SKIP: no GPU adapter in this environment — fire-target pixel proof not run");
        return;
    };
    let Some([dark_r, dark_g, dark_b, _dark_a]) = render_centre(false) else {
        eprintln!("SKIP: no GPU adapter in this environment — fire-target pixel proof not run");
        return;
    };

    // CLEARED: the dark clear colour reads near-black on every channel (no fire-target tile).
    assert!(
        dark_r < 40 && dark_g < 40 && dark_b < 40,
        "a cleared (non-target) cell must render the DARK clear colour: \
         got R={dark_r} G={dark_g} B={dark_b}",
    );

    // FIRE TARGET: the red tint composited over the dark clear is RED-DOMINANT — the R channel
    // clearly exceeds both G and B (the `FIRE_TARGET_TINT` is a red at ~50% alpha).
    assert!(
        i16::from(lit_r) - i16::from(lit_g) > 30 && i16::from(lit_r) - i16::from(lit_b) > 30,
        "the fire-target cell must render RED-DOMINANT (R clearly exceeds G and B): \
         got R={lit_r} G={lit_g} B={lit_b}",
    );
    // And NON-DARK vs the cleared clear: the red lifts the R channel well above the dark clear's.
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
