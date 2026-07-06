//! GTW-387 (C3 / D3): headless integration tests for the input-crate
//! `populate_reachable_overlay` system — it POPULATES the presenter-owned
//! `ReachableCells` for the `SelectedShooter` using the SAME `reachable_within` + fog
//! gate the path-preview and dispatch paths use, WIRED through the real plugin.
//!
//! POSITIVE — every assertion NAMES the cells or properties (not just "some cells exist"):
//!
//! - C3 (the producer + cross-storey): with a selected ganger, sim resources, and a
//!   stair link from L0 to L1, after settlement the `ReachableCells` resource contains L1
//!   cells — proving the input crate populates the cross-storey set correctly (including
//!   the GTW-387 B link-endpoint relaxation). Also proves it exactly matches a direct
//!   `reachable_within(...)` call (the §48 reuse guarantee).
//! - C3 (no selection → empty): with nothing selected the resource is cleared.
//! - C7 (WIRED): the systems run only via the registered plugin (no manual `add_systems`).
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)).

use bevy::{
    asset::AssetPlugin, input::ButtonInput, platform::collections::HashSet, prelude::*,
    scene::ScenePlugin,
};
use gdtf_battle_input::{GdtfBattleInputPlugin, PathPreviewTarget, SelectedShooter};
use gdtf_battle_presenter::{
    ActiveLevel, PathPreview, ReachableCells, ReachableOverlayEnabled, ViewMode,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    floor::FloorCostGrid,
    metric::MAX_LEVELS,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    pathfinder::{PlanningView, reachable_within},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    test_support::{SituationBuilder, key},
    tuning::CombatTuning,
    vertical::{VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
    visibility::{FactionRelation, SquadVisibility},
};
use gdtf_test_utils::advance_until;

/// The faction the player controls in these tests (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);

/// The selected ganger's TU budget — generous enough to reach the stair head and
/// at least one platform cell.
const BUDGET: Tu = Tu::new(200);

/// A full-vision `SquadVisibility` (the whole grid extent VISIBLE + EXPLORED) so every
/// cell is routable and the reachable set depends only on geometry + links.
fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "x/y are 0..60 and level is 0..MAX_LEVELS by the loop bounds"
                )]
                let c = CellLevel::new(Cell::new(x as i32, y as i32), Level::new(level));
                all.insert(c);
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

/// Map every occupant to `FactionRelation::Other` — the flat fixtures carry only the one
/// player ganger, so the resolver is never actually invoked for a blocking cell.
const fn all_other(_: Entity) -> FactionRelation {
    FactionRelation::Other
}

/// Build the `(CellLevel, CellLevel)` link endpoint pair from integer coordinates.
fn cell(x: i32, y: i32, level: u8) -> CellLevel {
    key(x, y, level)
}

/// Build a bidirectional stair graph connecting `foot` and `head`. Returns `None` on a
/// test-author wiring error (keeps the tests free of `unwrap`/`expect`).
fn stair_graph(foot: CellLevel, head: CellLevel) -> Option<VerticalLinkGraph> {
    use gdtf_battle_sim::vertical::LinkKind;
    let link = VerticalLink::new(foot, head, LinkKind::stair());
    let mut builder = SituationBuilder::new();
    builder = builder.slab_at(foot).slab_at(head);
    builder = builder.vertical_link(link);
    let situation = builder.build();
    build_vertical_link_graph(&situation).ok()
}

/// Build a focused headless app: `MinimalPlugins` + the real `GdtfBattleInputPlugin`,
/// plus the grids `populate_reachable_overlay` reads and the presenter-owned
/// `ReachableCells` + `ActiveLevel` (which the presenter plugin would normally init —
/// inserted here since this focused harness adds no renderer plugin).
///
/// GTW-450 — `overlay_enabled` seeds the `ReachableOverlayEnabled` flag RESOURCE directly
/// (NEVER via process-global `std::env`, the flaky-tests rule): the populate system's
/// `run_if` reads this resource's VALUE, so `false` keeps the producer inert (the shipping
/// default) and `true` opts it in.
fn reachable_app(links: VerticalLinkGraph, overlay_enabled: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(links);
    w.insert_resource(full_vision());
    let tuning = CombatTuning::default();
    w.insert_resource(FloorCostGrid::new(tuning.move_costs.open, []));
    w.insert_resource(tuning);
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(ActiveLevel::new(Level::new(0)));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    w.insert_resource(ViewMode::default());
    w.insert_resource(ReachableCells::cleared());
    w.insert_resource(ReachableOverlayEnabled::new(overlay_enabled));
    // GTW-450 C5(b) — the click-to-target route preview is the ONLY move feedback by default
    // and MUST stay working REGARDLESS of the overlay gating. Seed the presenter-owned
    // `PathPreview` so the always-on `populate_path_preview` system validates + runs here.
    w.insert_resource(PathPreview::cleared());
    app
}

/// C3 (no selection → cleared) — with no ganger selected the resource stays empty
/// (overlay flag ON so the producer actually runs; it still clears with no selection).
#[test]
fn clears_reachable_when_no_selection() {
    let links = VerticalLinkGraph::default();
    let mut app = reachable_app(links, true);

    // Seed a stale non-empty set, then leave SelectedShooter at its default (None).
    app.world_mut()
        .insert_resource(ReachableCells::new([(cell(5, 5, 0), Tu::new(4))]));

    app.update();

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "with no ganger selected the reachable set must be cleared (C3: no selection → empty)",
    );
}

/// GTW-450 C5(a) — flag OFF (the default): selecting a unit yields ZERO overlay cells.
/// The populate system `run_if`s the `ReachableOverlayEnabled` flag VALUE, so with the
/// flag `false` it NEVER runs and the read-seam stays empty even though a player ganger
/// IS selected with TU budget over a reachable grid.
///
/// PIN-DISCRIMINATION: this goes RED if the C3 flag `run_if` were dropped — then the
/// populate system would run regardless and fill the set, breaking the "no overlay by
/// default" contract. (Mirror of the flag-ON test below, which fills it.)
#[test]
fn no_overlay_cells_when_flag_off() {
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = stair_graph(foot, head) else {
        return; // test-author wiring error — guard without unwrap
    };

    // Flag OFF — the shipping default.
    let mut app = reachable_app(links, false);

    // Select a player ganger at the stair foot with a generous TU budget.
    let ganger = app
        .world_mut()
        .spawn((Position::new(foot), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    // Settle several updates so a (wrongly-running) populate system would have fired.
    for _ in 0..8 {
        app.update();
    }

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "with the overlay flag OFF (the default) a selected unit must yield ZERO overlay \
         cells (C5a: no overlay by default — the populate system is gated on the flag)",
    );
}

/// C3 / GTW-450 C5(c) — flag ON, the producer fills `ReachableCells` with EXACTLY the
/// `reachable_within` set and the set contains L1 cells (the stair head and beyond). This
/// exercises the GTW-387 B cross-storey link-endpoint relaxation.
///
/// PIN-DISCRIMINATION: this goes RED if `populate_reachable_overlay` stopped running (e.g.
/// the registration were dropped or the flag `run_if` inverted) — the set would stay empty
/// and the `advance_until` would time out. The flag is set ON via the RESOURCE directly.
#[test]
fn populates_reachable_cells_matching_reachable_within_including_l1() {
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = stair_graph(foot, head) else {
        return; // test-author wiring error — guard without unwrap
    };

    // Flag ON — opt the debug overlay producer in.
    let mut app = reachable_app(links, true);

    // Select the player ganger at the stair foot.
    let ganger = app
        .world_mut()
        .spawn((Position::new(foot), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    // Compute the independent baseline directly so we can compare.
    let expected: Vec<(CellLevel, Tu)> = {
        let grid = app.world().resource::<OccupancyGrid>().clone();
        let links = app.world().resource::<VerticalLinkGraph>().clone();
        let squad = app.world().resource::<SquadVisibility>().clone();
        let tuning = app.world().resource::<CombatTuning>().clone();
        let floor_costs = app.world().resource::<FloorCostGrid>().clone();
        let planning = PlanningView::new(&squad, all_other);
        reachable_within(
            foot,
            BUDGET,
            &grid,
            &links,
            &tuning,
            &floor_costs,
            gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
            &planning,
        )
    };

    // Settle: the populate system runs every Update; advance until the resource is populated.
    assert!(
        advance_until(
            &mut app,
            |app| !app.world().resource::<ReachableCells>().is_empty(),
            8,
        ),
        "the populate system must fill ReachableCells within 8 updates",
    );

    let populated: Vec<(CellLevel, Tu)> =
        app.world().resource::<ReachableCells>().cells().collect();

    // POSITIVE: the populated set contains the L1 link head — proving the cross-storey
    // reachable set includes L1 cells via the GTW-387 B relaxation.
    assert!(
        populated.iter().any(|(c, _)| *c == head),
        "the reachable set must include the L1 stair head ({head:?}) — the GTW-387 B \
         link-endpoint relaxation opens it even when UNSEEN; got {populated:?}",
    );

    // POSITIVE: the populated set exactly matches the direct `reachable_within` call —
    // proving the populate system reuses the sim search with no re-implementation.
    let populated_cells: Vec<CellLevel> = populated.iter().map(|(c, _)| *c).collect();
    let expected_cells: Vec<CellLevel> = expected.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        populated_cells, expected_cells,
        "the populated ReachableCells cells MUST exactly match a direct reachable_within call \
         (the populate system reuses the sim search); populated={populated_cells:?}, \
         expected={expected_cells:?}",
    );
}

/// GTW-450 C5(b) — the click-to-target route preview STILL works and is UNAFFECTED by the
/// overlay gating. With the overlay flag OFF (the default — no reachable overlay), selecting
/// a ganger and setting a move TARGET (the two-click flow's click-1) populates the
/// presenter-owned `PathPreview` with a route — the ONLY move highlight in shipping play (C4).
///
/// PIN-DISCRIMINATION: this goes RED if `populate_path_preview` / `PathPreview` broke (e.g. if
/// gating the overlay accidentally disturbed the always-on preview path) — the preview would
/// stay empty and the `advance_until` would time out. It uses the OFF flag precisely to prove
/// the preview is independent of the overlay opt-in.
#[test]
fn click_to_target_path_preview_still_works_with_overlay_off() {
    let links = VerticalLinkGraph::default();
    // Flag OFF — the shipping default; the route preview must still populate.
    let mut app = reachable_app(links, false);

    let start = cell(10, 10, 0);
    let goal = cell(14, 12, 0);

    // Select a player ganger and set the move target (the GTW-356 two-click click-1).
    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.world_mut()
        .insert_resource(PathPreviewTarget::new(goal));

    // Settle: the always-on `populate_path_preview` runs every Update.
    assert!(
        advance_until(
            &mut app,
            |app| !app.world().resource::<PathPreview>().is_empty(),
            8,
        ),
        "the click-to-target route preview must populate within 8 updates (C5b: it is the only \
         move feedback by default and stays working regardless of the overlay gating)",
    );

    let preview = app.world().resource::<PathPreview>();
    // POSITIVE: the route runs start..=goal — a real planned path, not an incidental cell.
    assert_eq!(
        preview.cells().first(),
        Some(&start),
        "the previewed route must start at the ganger's cell",
    );
    assert_eq!(
        preview.cells().last(),
        Some(&goal),
        "the previewed route must reach the clicked target",
    );

    // And the overlay itself stays EMPTY (flag OFF) — the two seams are independent.
    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "the reachable overlay stays empty with the flag OFF even while the preview is shown",
    );
}
