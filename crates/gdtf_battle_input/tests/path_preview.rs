//! GTW-358 (C1 / C2 / C6 / C7): headless integration tests for the input-crate
//! `populate_path_preview` system — it POPULATES the presenter-owned `PathPreview` for the
//! `SelectedShooter` → the `PathPreviewTarget` the SAME way `dispatch_move` plans a route,
//! REUSING `find_path` (no re-implemented routing), exposing the §48 `Path::total` cost, and
//! is WIRED in the input plugin.
//!
//! POSITIVE — every assertion NAMES the route cells + the cost (not "some preview exists"):
//!
//! - C1 / C6 (the producer): with a selected ganger + a target + the live grids + a
//!   full-vision `SquadVisibility`, after one update the `PathPreview` cells EXACTLY equal
//!   `find_path(start, goal, grids, &PlanningView::new(&squad, relation)).cells()` and its
//!   cost EXACTLY equals `path.total()` (the §48 bit-identity) — proving it reuses the sim
//!   search with the same `PlanningView` construction (so the previewed route + cost match
//!   what a commit accepts). Selection + target are never written into the sim.
//! - C2 (unreachable → empty): a target the ganger cannot reach (a goal walled off) yields an
//!   EMPTY preview.
//! - C1 (no target → empty): with NOTHING targeted the preview is empty (no route drawn).
//! - C7 (WIRED): the systems run only via the registered plugin (no manual `add_systems`).
//!
//! GTW-379 (FIRE→MOVE reset): engaging the fire-mode toggle (a `Changed<SelectedFireMode>`)
//! HIDES + RESETS a stale move-path preview — after the switch the `PathPreview` is empty and
//! `PathPreviewTarget` is cleared, and the preview only REAPPEARS after a fresh target select
//! (pin-discriminating — fails if the stale path persists); a normal move still plans + shows
//! its path. Driven through the same registered `GdtfBattleInputPlugin` systems.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)).

use bevy::{
    asset::AssetPlugin, input::ButtonInput, platform::collections::HashSet, prelude::*,
    scene::ScenePlugin,
};
use gdtf_battle_input::{
    GdtfBattleInputPlugin, PathPreviewTarget, SelectedFireMode, SelectedShooter,
};
use gdtf_battle_presenter::{ActiveLevel, PathPreview, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    floor::FloorCostGrid,
    metric::MAX_LEVELS,
    occupancy::{GRID_HEIGHT, GRID_WIDTH, TerrainKind},
    pathfinder::{PlanningView, find_path},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};
use gdtf_test_utils::advance_until;

/// The faction the player controls in these tests (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);

/// The selected ganger's TU budget — generous so affordability never gates the preview (the
/// preview shows the route regardless of budget; `find_path` plans, it does not charge).
const BUDGET: Tu = Tu::new(200);

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
/// grids `populate_path_preview` reads (all-Open occupancy, empty links, full-vision fog,
/// default tuning), the `BattleInProgress` + `PlayerFaction` gate witnesses, and the
/// presenter-owned `PathPreview` + `ActiveLevel` (which the presenter plugin would normally
/// init — inserted here since this focused harness adds no renderer plugin).
fn preview_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(VerticalLinkGraph::default());
    w.insert_resource(full_vision());
    let tuning = CombatTuning::default();
    // GTW-396: `populate_path_preview` (and the `dispatch_move` system it mirrors) reads
    // `Res<FloorCostGrid>` — seed a uniform grid at the default open cost so the input-
    // plugin systems validate (the preview tests use an all-Open occupancy grid, so every
    // step costs the default open rate).
    w.insert_resource(FloorCostGrid::new(tuning.move_costs.open, []));
    w.insert_resource(tuning);
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(ActiveLevel::new(Level::new(0)));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    w.insert_resource(ViewMode::default());
    w.insert_resource(PathPreview::cleared());
    app
}

/// Spawn a selected player-faction ganger at `start` with the BUDGET TU, and target `goal`.
fn select_and_target(app: &mut App, start: CellLevel, goal: CellLevel) {
    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.world_mut()
        .insert_resource(PathPreviewTarget::new(goal));
}

/// The route computed DIRECTLY (the same construction `dispatch_move` uses) — the independent
/// baseline the populate system's output must equal. `None` if the fixture does not route (the
/// caller asserts a route is present), keeping `expect`/`unwrap` out of the test (denied lints).
fn expected_route(app: &App, start: CellLevel, goal: CellLevel) -> Option<(Vec<CellLevel>, Tu)> {
    let grid = app.world().resource::<OccupancyGrid>();
    let links = app.world().resource::<VerticalLinkGraph>();
    let squad = app.world().resource::<SquadVisibility>();
    let tuning = app.world().resource::<CombatTuning>();
    let floor_costs = app.world().resource::<FloorCostGrid>();
    let planning = PlanningView::new(squad, all_other);
    find_path(
        start,
        goal,
        grid,
        links,
        tuning,
        floor_costs,
        gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok()
    .map(|path| (path.cells().to_vec(), path.total()))
}

/// C1 / C6 — with a selected ganger + a target the populate system fills `PathPreview` with
/// EXACTLY the `find_path` route cells the same way `dispatch_move` plans it, and the previewed
/// cost EXACTLY equals `Path::total()` (the §48 bit-identity). REUSE, no re-impl.
#[test]
fn populates_route_cells_and_cost_matching_find_path() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let goal = CellLevel::new(Cell::new(14, 12), Level::new(0));
    select_and_target(&mut app, start, goal);

    // The baseline is computed BEFORE the update so it reads the same grids the system will.
    let expected = expected_route(&app, start, goal);
    assert!(
        expected.is_some(),
        "the open-grid fixture must route from start to goal",
    );
    let (expected_cells, expected_cost) = expected.unwrap_or((Vec::new(), Tu::new(0)));
    assert!(
        expected_cells.first() == Some(&start) && expected_cells.last() == Some(&goal),
        "the fixture route must run start..=goal",
    );

    app.update();

    let preview = app.world().resource::<PathPreview>();
    // POSITIVE: the drawn preview cells == find_path cells() (NAMED start + goal endpoints).
    assert_eq!(
        preview.cells(),
        &expected_cells[..],
        "the populated preview cells must EXACTLY equal find_path cells() over the same \
         PlanningView (it reuses the sim search, no re-implemented routing)",
    );
    // POSITIVE: the exposed previewed cost == path.total() (the §48 bit-identity).
    assert_eq!(
        preview.cost(),
        expected_cost,
        "the previewed cost must EXACTLY equal Path::total() (the §48 cost GTW-355 charges)",
    );
}

/// C1 — with NOTHING targeted the populate system writes the empty preview (no route drawn),
/// even with a ganger selected.
#[test]
fn clears_preview_when_no_target() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(8, 8), Level::new(0));
    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    // Seed a stale non-empty preview, then leave the target empty (the default `None`).
    app.world_mut().insert_resource(PathPreview::new(
        vec![CellLevel::new(Cell::new(3, 3), Level::new(0))],
        Tu::new(4),
    ));
    app.world_mut()
        .insert_resource(PathPreviewTarget::cleared());

    app.update();

    let preview = app.world().resource::<PathPreview>();
    assert!(
        preview.is_empty(),
        "with no target set the preview clears to the empty route (no preview drawn)",
    );
}

/// C2 — an UNREACHABLE target (a goal sealed off by a ring of impassable walls) yields an EMPTY
/// preview (no-route → no preview).
#[test]
fn unreachable_target_yields_empty_preview() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    // Wall the goal cell off: a solid box of Wall around (20,20) on level 0 so no route reaches it.
    let goal = CellLevel::new(Cell::new(20, 20), Level::new(0));
    {
        let mut grid = app.world_mut().resource_mut::<OccupancyGrid>();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let at = CellLevel::new(Cell::new(20 + dx, 20 + dy), Level::new(0));
                grid.set_terrain(at, TerrainKind::Wall);
                // GTW-501: find_path reads the TAG-derived path-blocking surface, so a
                // hand-set wall must also mark that surface (mirroring the projection a
                // spawned wall entity's BlocksPathfinding marker yields).
                grid.set_path_blocking(at);
            }
        }
    }
    select_and_target(&mut app, start, goal);

    // Sanity: find_path agrees the goal is blocked over the same grids.
    let grid = app.world().resource::<OccupancyGrid>();
    let links = app.world().resource::<VerticalLinkGraph>();
    let squad = app.world().resource::<SquadVisibility>();
    let tuning = app.world().resource::<CombatTuning>();
    let floor_costs = app.world().resource::<FloorCostGrid>();
    let planning = PlanningView::new(squad, all_other);
    assert!(
        find_path(
            start,
            goal,
            grid,
            links,
            tuning,
            floor_costs,
            gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
            &planning
        )
        .is_err(),
        "the fixture must seal the goal off so find_path returns PathBlocked",
    );

    app.update();

    assert!(
        app.world().resource::<PathPreview>().is_empty(),
        "an unreachable target yields no preview (C2: PathBlocked → empty preview)",
    );
}

/// C6 — the populate system NEVER pushes the selection / target into the sim: they stay
/// input-crate resources, and the only output is the presenter-owned `PathPreview`.
#[test]
fn does_not_push_selection_or_target_into_sim() {
    let mut app = preview_app();
    let start = CellLevel::new(Cell::new(9, 9), Level::new(0));
    let goal = CellLevel::new(Cell::new(12, 9), Level::new(0));
    select_and_target(&mut app, start, goal);
    app.update();

    // Selection + target live ONLY on the input-crate resources; the produced output is the
    // presenter-owned `PathPreview` (a regression that mirrored either into a sim resource
    // would need a new sim resource — none exists; this pins the read-seam direction).
    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        Some(goal),
        "the target stays the input-crate PathPreviewTarget (never copied into the sim)",
    );
    assert!(
        !app.world().resource::<PathPreview>().is_empty(),
        "the populate output is the presenter-owned PathPreview, not a sim write",
    );
}

/// A fire mode DISTINCT from `SelectedFireMode::default()` so writing it trips
/// `Changed<SelectedFireMode>` — the FIRE→MOVE switch (`tu_percent` differs from the default 0).
const fn distinct_fire_mode() -> SelectedFireMode {
    SelectedFireMode::new(FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.5),
        ModeShots::new(1),
    ))
}

/// GTW-379 — switching FROM fire mode (the player engages the fire-mode toggle) HIDES + RESETS a
/// stale move-path preview: after a `Changed<SelectedFireMode>` the `PathPreview` is EMPTY and
/// `PathPreviewTarget` is cleared, and the preview only REAPPEARS after a FRESH target select.
/// PIN-DISCRIMINATING: it fails if the stale path persists across the switch.
#[test]
fn fire_mode_switch_hides_and_resets_stale_move_path() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let goal = CellLevel::new(Cell::new(14, 12), Level::new(0));
    select_and_target(&mut app, start, goal);

    // PRECONDITION (a normal move plans + shows its path): with a target set and NO fire-mode
    // change, the populate system draws the route. Settle past the first-update add frame.
    assert!(
        advance_until(
            &mut app,
            |app| !app.world().resource::<PathPreview>().is_empty(),
            8,
        ),
        "a normal move target must plan + show its path before any fire-mode switch",
    );
    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        Some(goal),
        "the move target is set before the switch",
    );

    // SWITCH to fire mode: the player engages the fire-mode toggle (a Changed<SelectedFireMode>).
    app.world_mut().insert_resource(distinct_fire_mode());
    app.update();

    // The stale move plan is RESET — the target cleared AND the preview hidden the same switch.
    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        None,
        "the FIRE→MOVE switch resets the move target to cleared",
    );
    assert!(
        app.world().resource::<PathPreview>().is_empty(),
        "the FIRE→MOVE switch HIDES the stale move-path preview (PIN: fails if it persists)",
    );

    // It STAYS hidden across further updates with no fresh target — the player must re-plan.
    app.update();
    assert!(
        app.world().resource::<PathPreview>().is_empty(),
        "the move-path preview stays hidden until a FRESH target is selected",
    );

    // RE-PLAN: a fresh target select brings the preview back (the two-click flow's click-1).
    let new_goal = CellLevel::new(Cell::new(7, 9), Level::new(0));
    app.world_mut()
        .insert_resource(PathPreviewTarget::new(new_goal));
    app.update();
    let preview = app.world().resource::<PathPreview>();
    assert!(
        !preview.is_empty(),
        "the preview REAPPEARS only after a fresh target select (a new plan is allowed)",
    );
    assert_eq!(
        preview.cells().last(),
        Some(&new_goal),
        "the re-planned preview routes to the NEW target (not the stale goal)",
    );
}
