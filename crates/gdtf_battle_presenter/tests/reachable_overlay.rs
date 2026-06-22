//! GTW-357 (C5 / C6): headless draw-LOGIC proof for the reachable-range overlay — the
//! POSITIVE in-engine state assertions the contract demands.
//!
//! C5 (HIGHLIGHT, positive): with a `ReachableOverlay` lit at NAMED cells, the presenter
//! draws a `Visibility::Visible`, world-positioned `ReachableTint` sprite at each NAMED
//! reachable cell ON the active storey, and an off-`ActiveLevel` reachable cell is NOT lit
//! (the hard cut, AC4) — and a cell NOT in the set has no lit tint over it (out-of-range is
//! dark). The assertion NAMES the lit cells that must appear and confirms the out-of-range /
//! off-storey cells are dark — "some highlight exists" is not sufficient.
//!
//! C6 (LABELS, positive): a TU-cost `Text2d` label EXISTS above each NAMED candidate cell on
//! the active storey and shows the CORRECT cost (the `(cell, Tu)` cost the overlay carries).
//! The assertion NAMES the cell + the expected value.
//!
//! This is the `fog_present.rs` pattern: a `DefaultPlugins`/`no_renderer` app with the real
//! `TopDownRendererPlugin`. The reachable-overlay draw systems gate ONLY on `BattleInProgress`
//! (a solid-tint sprite + a `Text2d`, no atlas), so the headless app drives the REAL draw path.
//! The `ReachableOverlay` resource is authored DIRECTLY via `app.world_mut()` in the test body
//! (the accepted headless idiom, `bevy-traps.md` #7 carve-out (a)) — standing in for the input
//! crate's populate system, which is unit-tested in `gdtf_battle_input` over the same resource.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::{Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Text2d,
    text::TextColor,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    CELL_PX, ReachableLabel, ReachableOverlay, ReachableTint, TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level, Tu};

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin`
/// (the `fog_present.rs` harness) plus a `BattleInProgress` witness so the battle-gated
/// reachable-overlay draw systems run.
fn overlay_app() -> App {
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
    // The live-battle witness the reachable-overlay draw systems gate on.
    app.insert_resource(BattleInProgress);
    // Warn-not-panic on a transient missing-resource gate race (the `fog_present.rs` precedent):
    // the focused harness opens `BattleInProgress` WITHOUT routing through the full `setup_battle`,
    // so some battle-gated `TopDownRendererPlugin` draw systems whose backing resources this test
    // does not seed simply warn-skip rather than panicking the run. The reachable-overlay draw
    // systems read only the `init_resource`-d `ReachableOverlay` + `ActiveLevel`, so they always run.
    app.set_error_handler(warn);
    app
}

/// Author the reachable overlay directly (standing in for the input populate system).
fn set_overlay(app: &mut App, cells: Vec<(CellLevel, Tu)>) {
    app.world_mut()
        .insert_resource(ReachableOverlay::new(cells));
}

/// Drive bounded `update()`s until at least one `ReachableTint` sprite has materialized (the
/// lazy pool spawn lands in the end-of-update command flush).
fn settle_tints(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&ReachableTint>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&ReachableTint>();
    q.iter(app.world()).next().is_some()
}

/// Whether a `ReachableTint` sprite is `Visible` at the planar world position of `cell` — the
/// presenter's lit-cell witness. Matches on the planar `(x, y)` (the layer-z + the internal
/// label lift are not part of the cell identity).
fn tint_visible_at(app: &mut App, cell: CellLevel) -> bool {
    let want = cell_to_world(Cell::new(cell.x, cell.y), Level::new(level_u8(cell)));
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &ReachableTint)>();
    q.iter(app.world()).any(|(t, vis, _)| {
        planar_eq(t.translation.x, want.x)
            && planar_eq(t.translation.y, want.y)
            && *vis == Visibility::Visible
    })
}

/// The TU-cost label string of any `Visible` `ReachableLabel` whose planar `(x, y)` sits over
/// `cell` (a small lift above the cell centre is allowed — the label sits ABOVE the cell), or
/// `None` if no visible label is over it.
fn label_over(app: &mut App, cell: CellLevel) -> Option<String> {
    let anchor = cell_to_world(Cell::new(cell.x, cell.y), Level::new(level_u8(cell)));
    let mut q = app.world_mut().query::<(
        &Transform,
        &Visibility,
        &Text2d,
        &TextColor,
        &ReachableLabel,
    )>();
    q.iter(app.world()).find_map(|(t, vis, text, ..)| {
        let over_cell = planar_eq(t.translation.x, anchor.x)
            // The label is lifted ABOVE the cell, so its y is >= the cell-centre y, within one cell.
            && (t.translation.y - anchor.y) >= 0.0
            && (t.translation.y - anchor.y) <= CELL_PX;
        (over_cell && *vis == Visibility::Visible).then(|| (**text).clone())
    })
}

/// Count of `Visible` `ReachableTint` sprites (so the hard cut + the surplus-hide can be pinned).
fn visible_tint_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &ReachableTint)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// The storey index of a `CellLevel`, narrowed to the `u8` a `Level` carries (the cells under
/// test are well within `u8`).
fn level_u8(cell: CellLevel) -> u8 {
    u8::try_from(cell.z).unwrap_or(u8::MAX)
}

/// Planar-position equality within float noise (world positions are exact `CELL_PX` multiples).
fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// C5 (positive) — the NAMED reachable cells on the active storey are LIT (a `Visible`,
/// correctly-positioned `ReachableTint`), the off-`ActiveLevel` reachable cell is NOT lit (the
/// hard cut), and a cell NOT in the reachable set has no lit tint (out-of-range is dark).
#[test]
fn reachable_cells_lit_offstorey_and_outofrange_dark() {
    let mut app = overlay_app();

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    // Two NAMED reachable cells on the active storey (level 0), one NAMED reachable cell on a
    // DIFFERENT storey (level 1 — must be hard-cut), and a NAMED cell NOT in the set.
    let near = CellLevel::new(Cell::new(10, 10), l0);
    let far_in_range = CellLevel::new(Cell::new(13, 10), l0);
    let off_storey = CellLevel::new(Cell::new(11, 10), l1);
    let out_of_range = CellLevel::new(Cell::new(40, 40), l0);

    set_overlay(
        &mut app,
        vec![
            (near, Tu::new(4)),
            (far_in_range, Tu::new(12)),
            (off_storey, Tu::new(8)),
        ],
    );
    assert!(
        settle_tints(&mut app),
        "the reachable tint sprites must have drawn"
    );

    // POSITIVE: both active-storey reachable cells are lit at their own world position.
    assert!(
        tint_visible_at(&mut app, near),
        "the NAMED reachable cell (10,10,L0) must be lit (a Visible, positioned tint)",
    );
    assert!(
        tint_visible_at(&mut app, far_in_range),
        "the NAMED in-range reachable cell (13,10,L0) must be lit",
    );

    // HARD CUT (AC4): the off-ActiveLevel reachable cell is NOT lit on the active storey.
    assert!(
        !tint_visible_at(&mut app, CellLevel::new(Cell::new(11, 10), l0)),
        "an off-storey reachable cell must NOT be drawn on the active storey (the hard cut)",
    );
    // Exactly the two active-storey cells are lit — the level-1 cell is hard-cut, not drawn.
    assert_eq!(
        visible_tint_count(&mut app),
        2,
        "only the two active-storey reachable cells are lit; the off-storey one is hard-cut",
    );

    // OUT-OF-RANGE is dark: a cell not in the reachable set has no lit tint over it.
    assert!(
        !tint_visible_at(&mut app, out_of_range),
        "a cell NOT in the reachable set (40,40,L0) must have no lit tint (out-of-range is dark)",
    );
}

/// C6 (positive) — a TU-cost label EXISTS above each NAMED candidate cell and shows the CORRECT
/// cost value (the `reachable_within` cost the overlay carries for that cell).
#[test]
fn tu_cost_labels_show_correct_value_above_named_cells() {
    let mut app = overlay_app();

    let l0 = Level::new(0);
    let cheap = CellLevel::new(Cell::new(10, 10), l0);
    let dear = CellLevel::new(Cell::new(15, 12), l0);

    set_overlay(&mut app, vec![(cheap, Tu::new(4)), (dear, Tu::new(20))]);
    assert!(settle_tints(&mut app), "the overlay must have drawn");

    // POSITIVE: the label over the cheap cell shows its EXACT cost, the dear cell its own.
    assert_eq!(
        label_over(&mut app, cheap).as_deref(),
        Some("4 TU"),
        "a TU-cost label above (10,10,L0) must show its 4-TU reachable cost",
    );
    assert_eq!(
        label_over(&mut app, dear).as_deref(),
        Some("20 TU"),
        "a TU-cost label above (15,12,L0) must show its 20-TU reachable cost",
    );
}

/// Clearing the overlay (nothing selected) HIDES every tint + label — mutate-not-respawn (the
/// pooled entities persist but go `Hidden`), so a deselect leaves no stale lit cell.
#[test]
fn clearing_overlay_hides_all_tints() {
    let mut app = overlay_app();
    let l0 = Level::new(0);
    let cell = CellLevel::new(Cell::new(7, 7), l0);

    set_overlay(&mut app, vec![(cell, Tu::new(4))]);
    assert!(settle_tints(&mut app), "the overlay must have drawn");
    assert!(
        tint_visible_at(&mut app, cell),
        "the reachable cell is lit before clear",
    );

    // Clear (deselect) — the pooled tint must hide, not linger lit.
    app.world_mut().insert_resource(ReachableOverlay::cleared());
    app.update();

    assert!(
        !tint_visible_at(&mut app, cell),
        "after the overlay is cleared, no tint stays lit (mutate-not-respawn hide)",
    );
    assert_eq!(
        visible_tint_count(&mut app),
        0,
        "a cleared overlay leaves zero lit tints (the pooled sprites are hidden)",
    );
}

/// Stale-entity guard: a tint pooled for a prior larger set must be HIDDEN when the set shrinks
/// (mutate-not-respawn surplus-hide), so a smaller reachable set never shows a leftover cell.
#[test]
fn shrinking_set_hides_surplus_pooled_tints() {
    let mut app = overlay_app();
    let l0 = Level::new(0);
    let a = CellLevel::new(Cell::new(5, 5), l0);
    let b = CellLevel::new(Cell::new(6, 5), l0);

    set_overlay(&mut app, vec![(a, Tu::new(4)), (b, Tu::new(8))]);
    assert!(settle_tints(&mut app), "the overlay must have drawn");
    assert_eq!(visible_tint_count(&mut app), 2, "both cells lit");

    // Shrink to one cell — the surplus pooled tint must hide.
    set_overlay(&mut app, vec![(a, Tu::new(4))]);
    app.update();
    assert_eq!(
        visible_tint_count(&mut app),
        1,
        "the shrunk set lights exactly one cell (the surplus pooled tint is hidden)",
    );
}
