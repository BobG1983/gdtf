//! GTW-373 (C4 (c), pixel proof): PIXEL-level proof that the stair-UP (index 29, the
//! GTW-373 split that supersedes the OQ-3 single-stair 77) and ladder (index 235) terrain
//! tiles actually RENDER non-empty and DISTINCT — vs a no-link (no-sprite) baseline that
//! renders the dark clear colour. (Covering at least one stair direction — the ascend
//! tile — proves the new split stair tile renders real pixels on the real GPU.)
//!
//! The green suite + the headless `vertical_link_draw.rs` state test prove the draw LOGIC
//! (the right sprite carries the right atlas index at the right cell), but a `Visible`,
//! correctly-indexed atlas sprite can still draw ZERO pixels (`bevy-traps.md` #8) — and
//! the dev screenshot capture is BROKEN. This closes that gap the
//! `reachable_overlay_readback.rs` / `fog_shader_readback.rs` way — a real-GPU headless
//! render-to-texture readback: it loads the SHIPPED terrain sheet, renders the stair / the
//! ladder / NO tile over a known dark clear to an offscreen `Image`, reads the centre
//! region back off the GPU, and asserts:
//!
//! - the stair-up tile (index 29) renders NON-EMPTY (distinctly brighter than the dark clear);
//! - the ladder tile (index 235) renders NON-EMPTY (distinctly brighter than the dark clear);
//! - the stair and ladder render DISTINCT from each other (different tiles, not the same
//!   pixels) — so the `LinkKind`-keyed index choice (C2) is visible at the pixel level.
//!
//! POSITIVE — it NAMES the stair-up / ladder indices and the no-link baseline and asserts
//! the pixels actually differ.
//!
//! Environment: needs a real GPU adapter (Metal on macOS) AND the SHIPPED terrain sheet
//! (loaded from the workspace `assets/`, the `fog_shader_readback.rs` load-bearing
//! `AssetPlugin` root). Single-threaded (a shared GPU lock) so two Bevy `App`s never init
//! Metal at once. No adapter (a GPU-less CI runner) → the harness SKIPS with a logged note.

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

/// The stair-UP tile index (GTW-373, supersedes the OQ-3 single-stair 77 — drawn where you
/// ascend) into the terrain sheet's atlas. Proving this tile renders covers the new split
/// stair (at least the ascend direction).
const STAIR_UP_INDEX: usize = 29;

/// The ladder tile index (user OQ-3 ruling, UNCHANGED) into the terrain sheet's atlas.
const LADDER_INDEX: usize = 235;

/// The terrain sheet grid: 16 columns x 22 rows of 16-px tiles (`SheetRole::Terrain`).
const TERRAIN_COLS: u32 = 16;
/// The terrain sheet's row count.
const TERRAIN_ROWS: u32 = 22;
/// The terrain sheet's per-tile edge in source px.
const TERRAIN_TILE_PX: u32 = 16;

/// The dark battlefield clear colour the no-link baseline reads as.
const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

/// The offscreen render-target edge (square). Matches one tile so the centred quad fills it.
const TARGET_PX: u32 = 16;

/// Max `app.update()` calls to wait for the readback observer to fire (the terrain image
/// must finish loading + uploading to the GPU first, so a generous budget).
const MAX_READBACK_UPDATES: usize = 240;

/// What a single render produced — the tile's mean colour + its brightest texel, or
/// `None` if no GPU.
#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    /// Whether the readback observer fired and populated this.
    captured:       bool,
    /// `[R, G, B, A]` sRGB-encoded (0..=255), averaged over the WHOLE tile — the
    /// DISTINCTNESS witness (a stair vs a ladder tile have different mean colours).
    rgba:           [u8; 4],
    /// The brightest single texel's channel-sum (`R+G+B`) across the WHOLE tile — the
    /// NON-EMPTY witness: even a mostly-dark tile (like the ladder) has at least one
    /// clearly-lit texel (a rung) far above the uniform dark clear, which a full-tile MEAN
    /// dilutes away. A no-sprite baseline has no such lit texel.
    max_brightness: u16,
}

/// Serialises the real-GPU tests in this binary (the `fog_shader_readback.rs` precedent).
static GPU_LOCK: Mutex<()> = Mutex::new(());

/// Take the process-wide GPU lock, recovering from a poisoned mutex.
fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The workspace-root `assets/` directory (manifest -> up two -> assets) — LOAD-BEARING so
/// the SHIPPED terrain sheet loads (the `fog_shader_readback.rs` `AssetPlugin` root).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Build a render-capable headless `App` on the real GPU (with the workspace asset root so
/// the terrain sheet loads), or `None` if no adapter.
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

    // With pipelined rendering disabled, the RenderApp staying on the main app proves a GPU
    // adapter exists; its absence means a GPU-less environment.
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Render the offscreen target with the terrain tile `index` present (`Some`) or absent
/// (`None`, the no-link baseline) over the dark clear, read the tile back, and return its
/// captured `(mean rgba, brightest-texel sum)` — or `None` if no GPU adapter exists.
fn render_tile(index: Option<usize>) -> Option<([u8; 4], u16)> {
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

    if let Some(index) = index {
        // Load the SHIPPED terrain sheet + build the 16x22 atlas layout, exactly as the
        // presenter's `load_topdown_atlases` does for `SheetRole::Terrain`.
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
        // The link tile sprite: the terrain atlas tile at `index`, sized to fill the target.
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

    // Read the whole target texture back; the observer captures the averaged centre region.
    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedPixel>| {
                let data = &trigger.event().data;
                // Rows are 256-aligned per the wgpu copy layout; index by row stride so the
                // padding never offsets the channel read.
                let row_stride = data.len() / TARGET_PX as usize;
                // Average the WHOLE tile (every texel of the target) so a tile whose detail
                // sits off-centre still reads its true mean brightness — a single-texel /
                // small-block sample can under-read a tile with sparse central coverage and
                // falsely read as "drew nothing".
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

    // Drive the FULL frame budget (not breaking early): the terrain image must finish
    // loading + uploading before the tile draws, and the first frames land before that. The
    // observer overwrites every frame, so the final read is the SETTLED draw.
    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates",
    );
    Some((captured.rgba, captured.max_brightness))
}

/// GTW-373 (C4 (c)) — the stair-up (29) and ladder (235) tiles render NON-EMPTY (each has a
/// clearly-lit texel far above the uniform dark baseline) and DISTINCT from each other
/// (their mean colours differ).
#[test]
fn stair_and_ladder_render_nonempty_and_distinct() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App`. On a
    // GPU-less runner `app.finish()` panics ("Unable to find a GPU!") before the in-build
    // `get_sub_app(RenderApp)?` guard, so skip here ahead of the app build.
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

    // BASELINE: the no-link target is the UNIFORM dark clear — its mean is near-black AND
    // its brightest texel is still dark (no tile drew, so no lit detail anywhere).
    assert!(
        baseline[0] < 40 && baseline[1] < 40 && baseline[2] < 40,
        "the no-link baseline must render the DARK clear colour: got {baseline:?}",
    );

    // NON-EMPTY: each tile has a clearly-lit texel far brighter than the baseline's
    // brightest — even the mostly-dark ladder has a lit rung. The brightest texel is the
    // right witness: a full-tile MEAN dilutes a sparse-detail tile toward the dark clear,
    // but a no-sprite baseline has NO lit texel at all, so this discriminates "drew the
    // tile" from "drew nothing".
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

    // DISTINCT: the stair and ladder render different mean colours (different tiles, not
    // the same pixels) — the LinkKind-keyed index choice (C2) is visible at the pixel level.
    let channel_diff = (i16::from(stair[0]) - i16::from(ladder[0])).abs()
        + (i16::from(stair[1]) - i16::from(ladder[1])).abs()
        + (i16::from(stair[2]) - i16::from(ladder[2])).abs();
    assert!(
        channel_diff > 5,
        "the stair-up (index 29) and ladder (index 235) tiles must render DISTINCT from \
         each other: stair mean={stair:?} vs ladder mean={ladder:?} (channel diff {channel_diff})",
    );
}
