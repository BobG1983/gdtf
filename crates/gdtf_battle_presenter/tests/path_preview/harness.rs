//! Shared `path_preview` fixture: the preview app, preview / vision authoring, and
//! the step + label probes.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{Text2d, Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    PathPreview, PathStepSprite, PathTargetLabel, TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, CellLevel, SquadVisibility, Tu};

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
pub(crate) const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// A `SquadVisibility` with every passed cell VISIBLE + EXPLORED — so §53 is not the variable
/// under test (the §53 VISIBLE-vs-EXPLORED dim is the pure-logic `preview_draws` unit test).
pub(crate) fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin` (the
/// `reachable_overlay.rs` harness) plus a `BattleInProgress` witness so the battle-gated
/// path-preview draw system runs.
pub(crate) fn preview_app() -> App {
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
    // Warn-not-panic on a transient missing-resource gate race (the `reachable_overlay.rs`
    // precedent): the focused harness opens `BattleInProgress` WITHOUT the full `setup_battle`,
    // so other battle-gated draw systems warn-skip rather than panicking. The path-preview draw
    // reads the `init_resource`-d `PathPreview` + `ActiveLevel` + the `SquadVisibility` the test
    // seeds, so it always runs here.
    app.set_error_handler(warn);
    app
}

/// Author the path preview + the squad fog directly (standing in for the input populate system).
pub(crate) fn set_preview(app: &mut App, cells: Vec<CellLevel>, cost: Tu) {
    app.world_mut().insert_resource(full_vision(&cells));
    app.world_mut()
        .insert_resource(PathPreview::new(cells, cost));
}

/// Drive bounded `update()`s until at least one `PathStepSprite` has materialized (the lazy
/// pool spawn lands in the end-of-update command flush).
pub(crate) fn settle_steps(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&PathStepSprite>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&PathStepSprite>();
    q.iter(app.world()).next().is_some()
}

/// Whether a `PathStepSprite` is `Visible` at the planar world position of `cell` — the
/// presenter's drawn-step witness (matches on the planar `(x, y)`; the layer-z is not part of
/// cell identity).
pub(crate) fn step_visible_at(app: &mut App, cell: CellLevel) -> bool {
    let (want_cell, want_level) = cell.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &PathStepSprite)>();
    q.iter(app.world()).any(|(t, vis, _)| {
        planar_eq(t.translation.x, want.x)
            && planar_eq(t.translation.y, want.y)
            && *vis == Visibility::Visible
    })
}

/// Count of `Visible` `PathStepSprite`s (so the hard cut + the surplus-hide can be pinned).
pub(crate) fn visible_step_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathStepSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// Drive bounded `update()`s until the SINGLE `PathTargetLabel` has materialized (GTW-368).
pub(crate) fn settle_label(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&PathTargetLabel>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&PathTargetLabel>();
    q.iter(app.world()).next().is_some()
}

/// The `(text, planar-position-matches-target, visible)` of the SINGLE target-cost label, if it
/// exists — the GTW-368 cost-on-target witness (the label is lifted ABOVE the cell, so it
/// matches the target's planar `x` and a `y` above the cell centre).
pub(crate) fn target_label_state(app: &mut App, target: CellLevel) -> Option<(String, bool, bool)> {
    let (want_cell, want_level) = target.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &Transform, &Visibility, &PathTargetLabel)>();
    q.iter(app.world()).next().map(|(text, t, vis, _)| {
        let over_target = planar_eq(t.translation.x, want.x) && t.translation.y > want.y;
        ((**text).clone(), over_target, *vis == Visibility::Visible)
    })
}

/// Count of `Visible` `PathTargetLabel`s — must never exceed ONE (the single target cost label).
pub(crate) fn visible_label_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathTargetLabel)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// Planar-position equality within float noise (world positions are exact `CELL_PX` multiples).
pub(crate) fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}
