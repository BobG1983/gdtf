//! GTW-596 (pixel proof): PIXEL-level proof that a cross-level Threat badge tile actually
//! renders RED-DOMINANT pixels at its known corner-slot world position — not just a `Visible`,
//! correctly-positioned entity (`bevy-traps.md` #8: a `Visible` sprite can still draw ZERO
//! pixels).
//!
//! The green suite + the `tests/cross_level_signals/` integration suite prove the draw LOGIC
//! (the badge tile is `Visible` at the right cell, tinted per-kind — `threat.rs`'s
//! `visible_tiles >= 1` assert), but none of them read a real GPU frame back. This closes that
//! gap the `fire_target_readback.rs` / `vertical_link_readback.rs` way: a real-GPU headless
//! render-to-texture readback drives the REAL `derive_cross_level_signals` +
//! `draw_cross_level_signals` systems (via the production `TopDownRendererPlugin`, the SAME
//! wiring `tests/cross_level_signals/threat.rs` exercises) — but on a real Metal adapter instead
//! of `backends: None` — over a squad-VISIBLE enemy two storeys above the active level, and
//! asserts:
//!
//! - with the enemy present, the badge's first corner-slot world position renders at least one
//!   texel clearly RED-DOMINANT (the `THREAT_TINT` background tile) — proven via the
//!   brightest-red-excess texel, not a whole-frame mean, so the co-drawn white label text (which
//!   shares the same slot) can never dilute the proof away (the `vertical_link_readback.rs`
//!   "brightest texel" precedent, adapted from raw brightness to red-dominance);
//! - with nothing squad-visible, the SAME slot renders the uniform dark clear and no texel is
//!   red-dominant.
//!
//! POSITIVE — it drives the shipped systems end to end (sim facts in, pixels out), not a
//! hand-rolled stand-in, so the rendered red is the shipped look.
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
    platform::collections::HashSet,
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
    ActiveLevel, CrossLevelBadgeTile, Layer, TopDownRendererPlugin, WORLD_RENDER_LAYER,
    cell_to_world_layered,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, LifeState, Position},
    visibility::SquadVisibility,
};
use gdtf_test_utils::gpu_adapter_probe;

/// The dark battlefield clear colour a cell with no cross-level signal reads as.
const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

/// The offscreen render-target edge (square) — tight around the badge tile's 6-world-px size
/// (`draw.rs::BADGE_SIZE_PX`) so the tile fills most of the captured frame and dilution by the
/// surrounding dark clear stays small.
const TARGET_PX: u32 = 8;

/// Max `app.update()` calls to wait for the derive + draw systems' settle and the readback
/// observer to fire.
const MAX_READBACK_UPDATES: usize = 90;

/// A captured readback frame: the whole-frame mean (the ABSENT-baseline "stayed dark" witness)
/// and the single reddest texel's red-excess (the PRESENT "a red-dominant pixel exists" witness,
/// immune to the co-drawn white label ink diluting a whole-frame mean).
#[derive(Resource, Default, Clone, Copy)]
struct CapturedFrame {
    /// Whether the readback observer fired and populated this.
    captured:       bool,
    /// Mean `[R, G, B, A]` sRGB bytes (0..=255) over the whole target.
    mean:           [u8; 4],
    /// The single texel with the highest `R - max(G, B)` across the whole target — a
    /// background-tile-only texel reads strongly positive; a dark-clear-only texel reads
    /// near zero/negative.
    max_red_excess: i16,
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
/// `TopDownRendererPlugin` (so the REAL `derive_cross_level_signals` / `draw_cross_level_signals`
/// run), or `None` if no adapter.
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
    // Both cross-level-signal systems are `run_if(resource_exists::<BattleInProgress>)`.
    app.insert_resource(BattleInProgress);
    // Warn-not-panic on a transient missing-resource gate race in the OTHER battle-gated draw
    // systems (this focused harness opens `BattleInProgress` without the full `setup_battle`,
    // the `fire_target_readback.rs` precedent).
    app.set_error_handler(warn);

    app.finish();
    app.cleanup();

    // With pipelined rendering disabled, the RenderApp sub-app stays on the main app — its
    // absence means no GPU adapter was created (a GPU-less environment).
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Drive the REAL derive + draw systems with a squad-VISIBLE enemy two storeys above the active
/// level (`present = true`) or with nothing squad-visible at all (`present = false`), render the
/// badge's known first corner-slot world position over the dark clear, read the frame back off
/// the GPU, and return its captured stats — or `None` if no GPU adapter exists.
fn render_badge_slot(present: bool) -> Option<CapturedFrame> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedFrame>();

    let active_cell = CellLevel::new(Cell::new(0, 0), Level::new(0));
    let enemy_cell = CellLevel::new(Cell::new(0, 0), Level::new(2));

    app.world_mut()
        .insert_resource(ActiveLevel::new(active_cell.level()));
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));

    if present {
        let visible: HashSet<CellLevel> = std::iter::once(enemy_cell).collect();
        app.world_mut()
            .insert_resource(SquadVisibility::new(visible.clone(), visible));
        app.world_mut()
            .spawn((Position::new(enemy_cell), Faction::new(1), LifeState::Alive));
    } else {
        app.world_mut()
            .insert_resource(SquadVisibility::new(HashSet::default(), HashSet::default()));
    }

    // The offscreen render target; COPY_SRC so the readback can copy it out.
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // The badge's FIRST corner-slot world position: `draw.rs`'s private `BADGE_SLOT_OFFSETS[0]`
    // (`(5.0, 5.0)`) mirrored here as a known offset (the `vertical_link_readback.rs`
    // `STAIR_UP_INDEX`-style precedent for hardcoding an internal, design-fixed magic number),
    // added to `cell_to_world_layered` at the `Layer::CrossLevelSignal` band.
    let badge_world = cell_to_world_layered(
        active_cell.cell(),
        active_cell.level(),
        Layer::CrossLevelSignal,
    ) + Vec3::new(5.0, 5.0, 0.0);

    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(DARK_CLEAR),
            ..default()
        },
        Transform::from_translation(badge_world),
        RenderTarget::Image(target_for_cam.into()),
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));

    // Read the whole target back; the observer captures the whole-frame mean AND the reddest
    // single texel.
    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedFrame>| {
                let data = &trigger.event().data;
                // Rows are 256-aligned per the wgpu copy layout; index by row stride so the
                // padding never offsets the channel read.
                let row_stride = data.len() / TARGET_PX as usize;
                let edge = TARGET_PX as usize;
                let mut sums = [0u32; 4];
                let mut count = 0u32;
                let mut max_red_excess = i16::MIN;
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
                            let excess = i16::from(r) - i16::from(g).max(i16::from(b));
                            max_red_excess = max_red_excess.max(excess);
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
                    captured.mean = [mean(sums[0]), mean(sums[1]), mean(sums[2]), mean(sums[3])];
                    captured.max_red_excess = max_red_excess;
                }
            },
        );

    // Drive the full frame budget (not breaking early): the first frames land before the derive
    // system's write, the draw system's lazy pool spawn + command flush, and the 2D pass having
    // drawn. The observer overwrites every frame, so after the budget the captured value is the
    // SETTLED steady-state draw (settle-before-read).
    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedFrame>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates"
    );
    if present {
        // Sanity: the real draw actually spawned the pooled tile when the enemy was visible.
        let mut q = app.world_mut().query::<&CrossLevelBadgeTile>();
        assert!(
            q.iter(app.world()).next().is_some(),
            "the REAL draw_cross_level_signals must have spawned the pooled CrossLevelBadgeTile",
        );
    }
    Some(captured)
}

/// GTW-596 (pixel proof) — the cross-level Threat badge's first corner-slot renders a texel
/// clearly RED-DOMINANT (the `THREAT_TINT` background tile) when a squad-VISIBLE enemy sits two
/// storeys above the active level; the SAME cell/slot renders no such red-dominant texel — and
/// reads the uniform dark clear — when nothing is squad-visible.
#[test]
fn threat_badge_slot_renders_red_dominant_absent_slot_stays_dark() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App`. On a GPU-less
    // runner `app.finish()` panics ("Unable to find a GPU!") before the in-build
    // `get_sub_app(RenderApp)?` guard, so skip here, ahead of the app build.
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — cross-level-signal pixel proof \
             not run"
        );
        return;
    }

    let Some(absent) = render_badge_slot(false) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — cross-level-signal pixel proof not run"
        );
        return;
    };
    let Some(present) = render_badge_slot(true) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — cross-level-signal pixel proof not run"
        );
        return;
    };

    // ABSENT: the whole capture is the uniform dark clear (no badge drawn) — every texel reads
    // near-black and no texel is red-dominant.
    assert!(
        absent.mean[0] < 40 && absent.mean[1] < 40 && absent.mean[2] < 40,
        "with no cross-level signal the badge slot must render the DARK clear colour: \
         got mean {:?}",
        absent.mean,
    );
    assert!(
        absent.max_red_excess < 20,
        "with no cross-level signal the badge slot must have NO red-dominant texel: \
         got max red-excess {}",
        absent.max_red_excess,
    );

    // PRESENT: at least one texel (the background tile, away from the white label ink) is
    // clearly RED-DOMINANT — far exceeding the absent baseline's near-zero red-excess.
    assert!(
        present.max_red_excess > absent.max_red_excess + 100,
        "a squad-visible enemy two storeys above must render a RED-DOMINANT badge texel: \
         present max red-excess {} vs absent {}",
        present.max_red_excess,
        absent.max_red_excess,
    );
}
