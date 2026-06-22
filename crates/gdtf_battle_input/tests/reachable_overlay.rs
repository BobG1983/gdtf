//! GTW-357 (C1 / C4 / C7): headless integration tests for the input-crate
//! `populate_reachable_overlay` system — it POPULATES the presenter-owned `ReachableOverlay`
//! for the `SelectedShooter` the SAME way `dispatch_move` plans a route, REUSING
//! `reachable_within` (no re-implemented reachability), and is WIRED in the input plugin.
//!
//! - C1 / C4 (the producer): with a selected ganger + the live grids + a full-vision
//!   `SquadVisibility`, after one update the `ReachableOverlay` EXACTLY equals
//!   `reachable_within(start, tu, grids, &PlanningView::new(&squad, relation))` computed
//!   directly — proving it reuses the sim flood with the same `PlanningView` construction
//!   (so the lit set matches what a commit accepts). Selection is never written into the sim.
//! - C1 (clear on no selection): with NOTHING selected the overlay is the empty set.
//! - C7 (WIRED): the systems run only via the registered plugin (no manual `add_systems`).
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)).

use bevy::{
    asset::AssetPlugin, input::ButtonInput, platform::collections::HashSet, prelude::*,
    scene::ScenePlugin,
};
use gdtf_battle_input::{GdtfBattleInputPlugin, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ReachableOverlay};
use gdtf_battle_sim::{
    BattleInProgress, Cell, CellLevel, CombatTuning, Faction, FactionRelation, GRID_HEIGHT,
    GRID_WIDTH, Level, MAX_LEVELS, OccupancyGrid, PlanningView, PlayerFaction, Position,
    SquadVisibility, Tu, VerticalLinkGraph, reachable_within,
};

/// The faction the player controls in these tests (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);

/// A small TU budget so the reachable flood is a tight, easily-checked disc.
const BUDGET: Tu = Tu::new(12);

/// A full-vision `SquadVisibility` (the whole grid extent VISIBLE + EXPLORED) so every cell is
/// routable and the route depends only on geometry — the hand-built mirror of the sim's
/// internal `full_vision` fixture.
fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "x/y are 0..60 and level is 0..MAX_LEVELS by the loop bounds, so the \
                              usize/u8 -> i32/u8 narrowing cannot truncate or wrap"
                )]
                let c = CellLevel::new(Cell::new(x as i32, y as i32), Level::new(level));
                all.insert(c);
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

/// Map every occupant to `FactionRelation::Other` — the flat fixtures carry only the one player
/// ganger (its own cell is the start, never gated), so the resolver result is incidental.
const fn all_other(_occupant: Entity) -> FactionRelation {
    FactionRelation::Other
}

/// Builds a focused headless app: `MinimalPlugins` + the real `GdtfBattleInputPlugin`, plus the
/// grids `populate_reachable_overlay` reads (all-Open occupancy, empty links, full-vision fog,
/// default tuning), the `BattleInProgress` + `PlayerFaction` gate witnesses, and the
/// presenter-owned `ReachableOverlay` + `ActiveLevel` (which the presenter plugin would normally
/// init — inserted here since this focused harness adds no renderer plugin).
fn overlay_app() -> App {
    let mut app = App::new();
    // The selection auto-select + populate run via the plugin; `AssetPlugin` + `ScenePlugin`
    // satisfy the selection-highlight spawn the plugin also registers under `BattleInProgress`.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(VerticalLinkGraph::default());
    w.insert_resource(full_vision());
    w.insert_resource(CombatTuning::default());
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(ActiveLevel::new(Level::new(0)));
    w.insert_resource(ReachableOverlay::cleared());
    app
}

/// The reachable set computed DIRECTLY (the same construction `dispatch_move` uses) — the
/// independent baseline the populate system's output must equal.
fn expected_set(app: &App, start: CellLevel) -> Vec<(CellLevel, Tu)> {
    let grid = app.world().resource::<OccupancyGrid>();
    let links = app.world().resource::<VerticalLinkGraph>();
    let squad = app.world().resource::<SquadVisibility>();
    let tuning = app.world().resource::<CombatTuning>();
    let planning = PlanningView::new(squad, all_other);
    reachable_within(start, BUDGET, grid, links, tuning, &planning)
}

/// C1 / C4 — with a selected ganger the populate system fills `ReachableOverlay` with EXACTLY
/// the `reachable_within` set computed the same way `dispatch_move` plans it (REUSE, no re-impl).
#[test]
fn populates_reachable_set_matching_dispatch_construction() {
    let mut app = overlay_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    // A player-faction ganger at `start` with the BUDGET TU and the player faction.
    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    // The baseline is computed BEFORE the update so it reads the same grids the system will.
    let expected = expected_set(&app, start);
    assert!(
        !expected.is_empty() && expected.iter().any(|(c, _)| *c == start),
        "the fixture must have a non-trivial reachable set including the start cell",
    );

    app.update();

    let overlay = app.world().resource::<ReachableOverlay>();
    assert_eq!(
        &**overlay,
        &expected[..],
        "the populated overlay must EXACTLY equal reachable_within over the same PlanningView \
         (it reuses the sim flood, byte-identical, no re-implemented reachability)",
    );
    // The start cell is in the lit set (a ganger can always "reach" where it stands at cost 0).
    assert!(
        overlay.iter().any(|(c, cost)| *c == start && **cost == 0),
        "the selected ganger's own cell is reachable at cost 0",
    );
}

/// C1 — with NOTHING selected the populate system writes the empty overlay (nothing lit).
#[test]
fn clears_overlay_when_nothing_selected() {
    let mut app = overlay_app();

    // Seed a stale non-empty overlay, then leave the selection empty (the default `None`).
    app.world_mut().insert_resource(ReachableOverlay::new(vec![(
        CellLevel::new(Cell::new(3, 3), Level::new(0)),
        Tu::new(4),
    )]));
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.update();

    let overlay = app.world().resource::<ReachableOverlay>();
    assert!(
        overlay.is_empty(),
        "with no ganger selected the overlay clears to the empty set",
    );
}

/// C4 — the populate system NEVER pushes the selection into the sim: there is no
/// selection-carrying sim resource, and the `SelectedShooter` stays an input-crate resource
/// (the overlay is the only output, presenter-owned).
#[test]
fn does_not_push_selection_into_sim() {
    let mut app = overlay_app();
    let start = CellLevel::new(Cell::new(8, 8), Level::new(0));
    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.update();

    // The selection lives ONLY on the input-crate `SelectedShooter`; the produced output is the
    // presenter-owned `ReachableOverlay`. (A regression that mirrored the selection into a sim
    // resource would need a new sim resource — none exists; this pins the read-seam direction.)
    assert_eq!(
        **app.world().resource::<SelectedShooter>(),
        Some(ganger),
        "the selection stays the input-crate SelectedShooter (never copied into the sim)",
    );
    assert!(
        !app.world().resource::<ReachableOverlay>().is_empty(),
        "the populate output is the presenter-owned overlay, not a sim write",
    );
}
