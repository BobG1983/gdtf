//! GTW-568 (C2, regression): a STEADY frame with unchanged draws must leave the surplus
//! HIDDEN pooled sprites' `Visibility` change ticks untouched.
//!
//! The pre-refactor hand-rolled pool loops ended with an unconditional
//! `*visibility = Visibility::Hidden` deref-write over every surplus pooled sprite —
//! re-marking every already-hidden sprite CHANGED every frame and re-triggering
//! visibility propagation each tick. The shared `draw_pool` walk (GTW-568) owns both
//! visibility flips through `set_if_neq`, so a steady frame writes nothing.
//!
//! Probed on a REAL migrated overlay — the path-preview step pool, driven end-to-end
//! through the real `TopDownRendererPlugin` draw system (the `tests/path_preview.rs`
//! harness). A probe system registered `.after(PresenterSystems::Draw)` counts, each
//! frame, the HIDDEN `PathStepSprite`s whose [`Ref<Visibility>::is_changed`] is `true`;
//! after the shrink-transition frame has settled, a steady frame must record ZERO.
//!
//! Proven discriminating against the pre-refactor code: this test was written BEFORE
//! the path-preview migration and observed FAILING (steady-frame count 2 — the two
//! surplus sprites re-dirtied) against the old unconditional write, then PASSING once
//! `draw_path_preview` moved onto `draw_pool`.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup, Update},
    asset::AssetPlugin,
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{
        Deref, DerefMut, DetectChanges, IntoScheduleConfigs, Query, Ref, ResMut, Resource,
        Visibility, With, default,
    },
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{PathPreview, PathStepSprite, PresenterSystems, TopDownRendererPlugin};
use gdtf_battle_sim::{
    prelude::{BattleInProgress, Cell, CellLevel, Level, Tu},
    visibility::SquadVisibility,
};

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// How many HIDDEN pooled path-step sprites had their `Visibility` marked changed on the
/// most recent frame — the C2 re-dirty witness (a named domain count, private inner,
/// written through the derived [`DerefMut`]).
#[derive(Resource, Default, Deref, DerefMut)]
struct HiddenRedirtyCount(usize);

/// Per-frame probe (`.after(PresenterSystems::Draw)`): count the HIDDEN pooled
/// `PathStepSprite`s whose `Visibility` is change-flagged since this probe last ran —
/// i.e. surplus pooled sprites the draw system re-dirtied THIS frame.
fn record_hidden_redirty(
    pooled: Query<Ref<Visibility>, With<PathStepSprite>>,
    mut count: ResMut<HiddenRedirtyCount>,
) {
    **count = pooled
        .iter()
        .filter(|vis| **vis == Visibility::Hidden && vis.is_changed())
        .count();
}

/// A `SquadVisibility` with every passed cell VISIBLE + EXPLORED — squad fog is not the
/// variable under test.
fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin`
/// (the `path_preview.rs` harness) plus the per-frame re-dirty probe after the draw set.
fn probe_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    app.init_resource::<HiddenRedirtyCount>();
    // The probe reads the pooled sprites AFTER the draw wrote them this frame, so its
    // `Ref::is_changed` (relative to the probe's own last run) sees exactly the frame's
    // draw-system writes.
    app.add_systems(Update, record_hidden_redirty.after(PresenterSystems::Draw));
    // Warn-not-panic on a transient missing-resource gate race (the `path_preview.rs`
    // precedent): the focused harness opens `BattleInProgress` WITHOUT the full
    // `setup_battle`, so other battle-gated draw systems warn-skip rather than panicking.
    app.set_error_handler(warn);
    app
}

/// Author the path preview + the squad fog directly (standing in for the input populate
/// system).
fn set_preview(app: &mut App, cells: Vec<CellLevel>, cost: Tu) {
    app.world_mut().insert_resource(full_vision(&cells));
    app.world_mut()
        .insert_resource(PathPreview::new(cells, cost));
}

/// Drive bounded `update()`s until `want` pooled `PathStepSprite`s exist (the lazy pool
/// spawn lands in the end-of-update command flush).
fn settle_pool_size(app: &mut App, want: usize) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&PathStepSprite>();
        if q.iter(app.world()).count() >= want {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&PathStepSprite>();
    q.iter(app.world()).count() >= want
}

/// Count of HIDDEN pooled `PathStepSprite`s — the surplus witness.
fn hidden_step_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathStepSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Hidden)
        .count()
}

/// GTW-568 C2 — after the route shrinks (leaving surplus hidden pooled sprites) and the
/// hide transition settles, a STEADY frame with unchanged draws re-dirties ZERO hidden
/// sprites: their `Visibility` change ticks stay untouched (`Ref::is_changed` false at
/// the probe). The pre-refactor unconditional `*visibility = Hidden` write fails this
/// with a steady-frame count of 2.
#[test]
fn steady_frame_leaves_surplus_hidden_visibility_ticks_untouched() {
    let mut app = probe_app();
    let l0 = Level::new(0);
    let a = CellLevel::new(Cell::new(5, 5), l0);
    let b = CellLevel::new(Cell::new(6, 5), l0);
    let c = CellLevel::new(Cell::new(7, 5), l0);

    // Grow the pool to three steps, then shrink the route to one cell so TWO pooled
    // sprites become surplus (hidden).
    set_preview(&mut app, vec![a, b, c], Tu::new(12));
    assert!(
        settle_pool_size(&mut app, 3),
        "the three route step sprites must have pooled",
    );
    set_preview(&mut app, vec![a], Tu::new(4));
    // The shrink TRANSITION frame: the two surplus sprites are genuinely hidden here (a
    // real write, expected to be change-flagged on this frame).
    app.update();
    assert_eq!(
        hidden_step_count(&mut app),
        2,
        "the shrunk route leaves two surplus pooled sprites hidden",
    );

    // STEADY frame: the draws are unchanged, the surplus sprites are already hidden — the
    // draw system must not re-dirty them (set_if_neq owns the flips, GTW-568 C2).
    app.update();
    let redirtied = app.world().resource::<HiddenRedirtyCount>();
    assert_eq!(
        **redirtied, 0,
        "a steady frame with unchanged draws must leave every surplus hidden pooled \
         sprite's Visibility change ticks untouched (the pre-refactor unconditional \
         `*visibility = Hidden` write re-dirtied them every frame)",
    );
}
