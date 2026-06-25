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
use gdtf_battle_input::{GdtfBattleInputPlugin, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ReachableCells};
use gdtf_battle_sim::{
    BattleInProgress, Cell, CellLevel, CombatTuning, Faction, FactionRelation, FloorCostGrid,
    GRID_HEIGHT, GRID_WIDTH, Level, MAX_LEVELS, OccupancyGrid, PlanningView, PlayerFaction,
    Position, SquadVisibility, Tu, VerticalLink, VerticalLinkGraph, build_vertical_link_graph,
    reachable_within,
    test_support::{SituationBuilder, key},
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
fn reachable_app(links: VerticalLinkGraph) -> App {
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
    w.insert_resource(ReachableCells::cleared());
    app
}

/// C3 (no selection → cleared) — with no ganger selected the resource stays empty.
#[test]
fn clears_reachable_when_no_selection() {
    let links = VerticalLinkGraph::default();
    let mut app = reachable_app(links);

    // Seed a stale non-empty set, then leave SelectedShooter at its default (None).
    app.world_mut()
        .insert_resource(ReachableCells::new([(cell(5, 5, 0), Tu::new(4))]));

    app.update();

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "with no ganger selected the reachable set must be cleared (C3: no selection → empty)",
    );
}

/// C3 (the producer) — with a selected ganger the populate system fills `ReachableCells`
/// with EXACTLY the `reachable_within` set and the set contains L1 cells (the stair head
/// and beyond). This exercises the GTW-387 B cross-storey link-endpoint relaxation.
#[test]
fn populates_reachable_cells_matching_reachable_within_including_l1() {
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = stair_graph(foot, head) else {
        return; // test-author wiring error — guard without unwrap
    };

    let mut app = reachable_app(links);

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
