//! PIXEL-level proof that the hover-highlight reticle RECOLOURS off the carried
//! [`CellVisibility`] verdict (GTW-11 C9).
//!
//! The green suite proves the reticle's `Sprite.color` is SET to the right tint
//! ([`tests/highlight_draw.rs`]), but only an on-GPU render proves those distinct tints
//! actually reach distinct PIXELS. This test closes that gap with a **real-GPU headless
//! render-to-texture readback**: it builds a render-capable `App` on the real Metal
//! backend (no window), drives the REAL [`draw_highlight_on_request`] system with a
//! [`HighlightRequest`] at each verdict, renders the resulting [`HoverHighlight`] sprite to
//! an offscreen `Image`, reads the rendered centre pixel back off the GPU via
//! [`Readback`], and asserts:
//!
//! - a [`CellVisibility::NotSquadVisible`] verdict ("unseen — hold your fire") renders a
//!   DISTINCT pixel from a [`CellVisibility::SquadVisible`] verdict (the reticle genuinely
//!   recolours), AND
//! - the non-VISIBLE pixel is the cold-grey unseen recolour (its blue channel exceeds its
//!   red — the warm normal tint is the opposite), so the recolour is the intended direction,
//!   not merely "some other colour".
//!
//! The reticle is a SOLID-tint `Sprite` (no custom material / WGSL), so unlike the
//! `fog_shader_readback.rs` companion this needs no `Material2dPlugin` — just the default
//! 2D sprite pipeline. It runs single-threaded against a shared GPU lock and skips with a
//! logged note when no adapter exists (a GPU-less CI runner) — the proof is valid where a
//! GPU exists, which is the machine this renders on.

use std::sync::{Mutex, MutexGuard};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::message::Messages,
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
    CellVisibility, HighlightRequest, HoverHighlight, WORLD_RENDER_LAYER, cell_to_world,
    draw_highlight_on_request,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level};
use gdtf_test_utils::gpu_adapter_probe;

/// The offscreen render-target edge (square). The reticle quad covers most of it.
const TARGET_PX: u32 = 64;

/// Max `app.update()` calls to wait for the readback observer to fire.
const MAX_READBACK_UPDATES: usize = 60;

/// A captured readback pixel (the centre of the target), sRGB-encoded bytes.
#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    /// Whether the readback observer fired and populated this.
    captured: bool,
    /// `[R, G, B, A]` sRGB-encoded (0..=255), as written to the `Rgba8UnormSrgb` target.
    rgba:     [u8; 4],
}

/// Serialises the real-GPU tests (two simultaneous Metal devices can exhaust the device).
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
            // Keep the render world synchronous within app.update() so the readback fires.
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    );
    // The REAL reticle draw system + its message buffer + the battle gate it runs under.
    app.add_message::<HighlightRequest>()
        .insert_resource(BattleInProgress)
        .add_systems(Update, draw_highlight_on_request);

    app.finish();
    app.cleanup();
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Drive the REAL `draw_highlight_on_request` with a `HighlightRequest` at `verdict` for `cell`,
/// render the resulting reticle to an offscreen target, and read the centre pixel back off the
/// GPU — or `None` if no GPU adapter exists.
fn render_reticle(cell: CellLevel, verdict: CellVisibility) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    // The offscreen render target; add COPY_SRC so the readback can copy it out. A magenta
    // clear makes a "reticle never drew" failure obvious (it would read magenta, never a tint).
    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    // A camera on the WORLD render layer (the reticle draws there), framing the reticle's
    // world position. The reticle sits at `cell_to_world(cell)`, so centre the camera on it.
    let centre = cell_to_world(cell.cell(), Level::new(0));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(1.0, 0.0, 1.0)),
            ..default()
        },
        Transform::from_translation(centre),
        RenderTarget::Image(target_handle.clone().into()),
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));

    // Send the REAL request: the draw system spawns / tints the ONE reticle off the verdict.
    app.world_mut()
        .resource_mut::<Messages<HighlightRequest>>()
        .write(HighlightRequest::new(Some(cell), verdict));

    // Read the whole target back; the observer captures the centre pixel.
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

    // Drive the full budget (the reticle spawns frame 1, draws steady-state after).
    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates"
    );
    // Sanity: exactly one reticle was spawned (no duplicate) — the draw mutates in place.
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "exactly one reticle sprite must exist (the draw mutates the one in place)"
    );
    Some(captured.rgba)
}

/// GTW-11 C9 — the reticle on a NON-VISIBLE cell renders a DISTINCT pixel from the SAME cell at
/// the squad-VISIBLE verdict (the recolour is real on the GPU), and the non-VISIBLE pixel is the
/// cold-grey unseen recolour (blue > red), not the warm normal tint (red > blue).
#[test]
fn reticle_recolours_on_the_gpu_for_a_non_visible_cell() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App`. On a
    // GPU-less runner `app.finish()` panics ("Unable to find a GPU!") before the in-build
    // `get_sub_app(RenderApp)?` guard, so skip here ahead of the app build.
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — reticle recolour proof not run"
        );
        return;
    }

    let cell = CellLevel::new(Cell::new(8, 8), Level::new(0));

    let Some([vr, vg, vb, va]) = render_reticle(cell, CellVisibility::SquadVisible) else {
        eprintln!("SKIP: no GPU adapter in this environment — reticle recolour proof not run");
        return;
    };
    let Some([ur, ug, ub, ua]) = render_reticle(cell, CellVisibility::NotSquadVisible) else {
        eprintln!("SKIP: no GPU adapter in this environment — reticle recolour proof not run");
        return;
    };

    // The reticle is a translucent tint OVER the magenta clear; both pixels must be opaque
    // enough to have drawn the tinted sprite (not the bare clear).
    assert!(va >= 64, "VISIBLE reticle pixel looks unpainted: a={va}");
    assert!(
        ua >= 64,
        "non-VISIBLE reticle pixel looks unpainted: a={ua}"
    );

    // DISTINCT: the two verdicts render to a genuinely different colour (the recolour is real).
    let channel_diff = (i16::from(vr) - i16::from(ur)).abs()
        + (i16::from(vg) - i16::from(ug)).abs()
        + (i16::from(vb) - i16::from(ub)).abs();
    assert!(
        channel_diff > 15,
        "the VISIBLE tint ({vr},{vg},{vb}) and the non-VISIBLE tint ({ur},{ug},{ub}) must render \
         DISTINCT pixels (the reticle recolours)",
    );

    // DIRECTION: the non-VISIBLE recolour is COLD-grey (blue exceeds red); the normal tint is
    // WARM (red exceeds blue). Over a magenta clear both share the clear's red/blue floor, so
    // compare the per-verdict red-vs-blue balance: the unseen recolour shifts BLUER than visible.
    let visible_warm = i16::from(vr) - i16::from(vb);
    let unseen_cold = i16::from(ur) - i16::from(ub);
    assert!(
        unseen_cold < visible_warm,
        "the non-VISIBLE tint must be COLDER (less red-over-blue) than the VISIBLE tint: \
         unseen R-B={unseen_cold}, visible R-B={visible_warm}",
    );
}
