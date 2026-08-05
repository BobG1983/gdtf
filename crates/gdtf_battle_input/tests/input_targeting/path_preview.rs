//! Path preview: route cells and cost match `find_path`; fire-mode switch clears stale preview.

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
    pathfinder::{MoveGrids, PlanningView, find_path},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};
use gdtf_test_utils::advance_until;

const PLAYER_FACTION: Faction = Faction::new(0);

const BUDGET: Tu = Tu::new(200);

fn full_vision() -> SquadVisibility {
    let (Ok(width), Ok(height)) = (i32::try_from(GRID_WIDTH), i32::try_from(GRID_HEIGHT)) else {
        return SquadVisibility::default();
    };
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..height {
            for x in 0..width {
                all.insert(CellLevel::new(Cell::new(x, y), Level::new(level)));
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

const fn all_other(_occupant: Entity) -> FactionRelation {
    FactionRelation::Other
}

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
    w.insert_resource(FloorCostGrid::new(tuning.move_costs.open, []));
    w.insert_resource(tuning);
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(ActiveLevel::new(Level::new(0)));
    w.insert_resource(ViewMode::default());
    w.insert_resource(PathPreview::cleared());
    app
}

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
        MoveGrids {
            occupancy: grid,
            links,
            floor_costs,
            tuning,
        },
        gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok()
    .map(|path| (path.cells().to_vec(), path.total()))
}

#[test]
fn populates_route_cells_and_cost_matching_find_path() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let goal = CellLevel::new(Cell::new(14, 12), Level::new(0));
    select_and_target(&mut app, start, goal);

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
    assert_eq!(
        preview.cells(),
        &expected_cells[..],
        "the populated preview cells must EXACTLY equal find_path cells() over the same \
         PlanningView (it reuses the sim search, no re-implemented routing)",
    );
    assert_eq!(
        preview.cost(),
        expected_cost,
        "the previewed cost must EXACTLY equal Path::total() (the §48 cost charges)",
    );
}

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

#[test]
fn unreachable_target_yields_empty_preview() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
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
                grid.set_path_blocking(at);
            }
        }
    }
    select_and_target(&mut app, start, goal);

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
            MoveGrids {
                occupancy: grid,
                links,
                floor_costs,
                tuning,
            },
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

#[test]
fn does_not_push_selection_or_target_into_sim() {
    let mut app = preview_app();
    let start = CellLevel::new(Cell::new(9, 9), Level::new(0));
    let goal = CellLevel::new(Cell::new(12, 9), Level::new(0));
    select_and_target(&mut app, start, goal);
    app.update();

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

const fn distinct_fire_mode() -> SelectedFireMode {
    SelectedFireMode::new(FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.5),
        ModeShots::new(1),
    ))
}

#[test]
fn fire_mode_switch_hides_and_resets_stale_move_path() {
    let mut app = preview_app();

    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let goal = CellLevel::new(Cell::new(14, 12), Level::new(0));
    select_and_target(&mut app, start, goal);

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

    app.world_mut().insert_resource(distinct_fire_mode());
    app.update();

    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        None,
        "the FIRE→MOVE switch resets the move target to cleared",
    );
    assert!(
        app.world().resource::<PathPreview>().is_empty(),
        "the FIRE→MOVE switch HIDES the stale move-path preview (PIN: fails if it persists)",
    );

    app.update();
    assert!(
        app.world().resource::<PathPreview>().is_empty(),
        "the move-path preview stays hidden until a FRESH target is selected",
    );

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
