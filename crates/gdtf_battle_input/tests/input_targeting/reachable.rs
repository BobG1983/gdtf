//! Reachable overlay: cells match `reachable_within`; clears without selection.
use bevy::{
    asset::AssetPlugin, input::ButtonInput, platform::collections::HashSet, prelude::*,
    scene::ScenePlugin,
};
use gdtf_battle_input::{GdtfBattleInputPlugin, PathPreviewTarget, SelectedShooter};
use gdtf_battle_presenter::{
    ActiveLevel, PathPreview, ReachableCells, ReachableOverlayEnabled, ViewMode,
};
use gdtf_battle_sim::{
    acts::{DismountSurcharge, dismount_surcharge, seat_departure},
    battle::PlayerFaction,
    emplacement::{EmplacementEntrySides, EmplacementFacing, Mounted, MountedBy},
    entity::TerrainCell,
    floor::FloorCostGrid,
    metric::MAX_LEVELS,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    pathfinder::{Departure, MoveGrids, PlanningView, reachable_within},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    terrain::facing::TerrainFacing,
    test_support::{SituationBuilder, key},
    tuning::CombatTuning,
    vertical::{VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
    visibility::{FactionRelation, SquadVisibility},
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

const fn all_other(_: Entity) -> FactionRelation {
    FactionRelation::Other
}

fn cell(x: i32, y: i32, level: u8) -> CellLevel {
    key(x, y, level)
}

fn stair_graph(foot: CellLevel, head: CellLevel) -> Option<VerticalLinkGraph> {
    use gdtf_battle_sim::vertical::LinkKind;
    let link = VerticalLink::new(foot, head, LinkKind::stair());
    let mut builder = SituationBuilder::new();
    builder = builder.slab_at(foot).slab_at(head);
    builder = builder.vertical_link(link);
    let situation = builder.build();
    build_vertical_link_graph(&situation).ok()
}

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
    w.insert_resource(ButtonInput::<KeyCode>::default());
    w.insert_resource(ActiveLevel::new(Level::new(0)));
    w.insert_resource(ViewMode::default());
    w.insert_resource(ReachableCells::cleared());
    w.insert_resource(ReachableOverlayEnabled::new(overlay_enabled));
    w.insert_resource(PathPreview::cleared());
    app
}

#[test]
fn clears_reachable_when_no_selection() {
    let links = VerticalLinkGraph::default();
    let mut app = reachable_app(links, true);

    app.world_mut()
        .insert_resource(ReachableCells::new([(cell(5, 5, 0), Tu::new(4))]));

    app.update();

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "with no ganger selected the reachable set must be cleared (C3: no selection → empty)",
    );
}

#[test]
fn no_overlay_cells_when_flag_off() {
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = stair_graph(foot, head) else {
        return;
    };

    let mut app = reachable_app(links, false);

    let ganger = app
        .world_mut()
        .spawn((Position::new(foot), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    for _ in 0..8 {
        app.update();
    }

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "with the overlay flag OFF (the default) a selected unit must yield ZERO overlay \
         cells (C5a: no overlay by default — the populate system is gated on the flag)",
    );
}

#[test]
fn populates_reachable_cells_matching_reachable_within_including_l1() {
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = stair_graph(foot, head) else {
        return;
    };

    let mut app = reachable_app(links, true);

    let ganger = app
        .world_mut()
        .spawn((Position::new(foot), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    let expected: Vec<(CellLevel, Tu)> = {
        let grid = app.world().resource::<OccupancyGrid>().clone();
        let links = app.world().resource::<VerticalLinkGraph>().clone();
        let squad = app.world().resource::<SquadVisibility>().clone();
        let tuning = app.world().resource::<CombatTuning>().clone();
        let floor_costs = app.world().resource::<FloorCostGrid>().clone();
        let planning = PlanningView::new(&squad, all_other);
        reachable_within(
            &Departure::anywhere(foot),
            BUDGET,
            DismountSurcharge::NONE,
            MoveGrids {
                occupancy:   &grid,
                links:       &links,
                floor_costs: &floor_costs,
                tuning:      &tuning,
            },
            gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
            &planning,
        )
    };

    advance_until(&mut app, |app| {
        !app.world().resource::<ReachableCells>().is_empty()
    });

    let populated: Vec<(CellLevel, Tu)> =
        app.world().resource::<ReachableCells>().cells().collect();

    assert!(
        populated.iter().any(|(c, _)| *c == head),
        "the reachable set must include the L1 stair head ({head:?}) — the link-endpoint \
         relaxation opens it even when UNSEEN; got {populated:?}",
    );

    let populated_cells: Vec<CellLevel> = populated.iter().map(|(c, _)| *c).collect();
    let expected_cells: Vec<CellLevel> = expected.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        populated_cells, expected_cells,
        "the populated ReachableCells cells MUST exactly match a direct reachable_within call \
         (the populate system reuses the sim search); populated={populated_cells:?}, \
         expected={expected_cells:?}",
    );
}

#[test]
fn a_mounted_selection_reaches_only_what_its_seat_lets_it_leave_by() {
    let mut app = reachable_app(VerticalLinkGraph::default(), true);

    let seat_cell = cell(5, 5, 0);
    let step = *app.world().resource::<CombatTuning>().move_costs.open;
    let budget = Tu::new(step.saturating_mul(4));

    let ganger = app
        .world_mut()
        .spawn((Position::new(seat_cell), budget, PLAYER_FACTION))
        .id();
    let sides = EmplacementEntrySides::new(vec![TerrainFacing::North]);
    let facing = EmplacementFacing::new(TerrainFacing::default());
    app.world_mut().spawn((
        TerrainCell::new(seat_cell),
        MountedBy::new(ganger),
        sides.clone(),
        facing,
    ));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    assert!(
        app.world().get::<Mounted>(ganger).is_some(),
        "the seat's MountedBy must give the ganger its Mounted before the overlay runs",
    );

    let departure = seat_departure(seat_cell, seat_cell, Some(&sides), Some(&facing));
    let expected = direct_reachable(&app, ganger, &departure, budget);
    let unconstrained = direct_reachable(&app, ganger, &Departure::anywhere(seat_cell), budget);

    advance_until(&mut app, |app| {
        !app.world().resource::<ReachableCells>().is_empty()
    });

    let populated: Vec<(CellLevel, Tu)> =
        app.world().resource::<ReachableCells>().cells().collect();
    assert_eq!(
        populated, expected,
        "the overlay must offer exactly what reachable_within offers for the seat's departure \
         and exit; populated={populated:?}, expected={expected:?}",
    );

    assert!(
        unconstrained
            .iter()
            .any(|(cell, _)| !expected.iter().any(|(offered, _)| offered == cell)),
        "the fixture must discriminate: leaving anywhere must offer a cell the seat's one entry \
         side does not reach; anywhere={unconstrained:?}, seated={expected:?}",
    );
}

/// The reachable set a direct `reachable_within` call gives this ganger for `departure`.
fn direct_reachable(
    app: &App,
    ganger: Entity,
    departure: &Departure,
    budget: Tu,
) -> Vec<(CellLevel, Tu)> {
    let world = app.world();
    let grid = world.resource::<OccupancyGrid>().clone();
    let links = world.resource::<VerticalLinkGraph>().clone();
    let squad = world.resource::<SquadVisibility>().clone();
    let tuning = world.resource::<CombatTuning>().clone();
    let floor_costs = world.resource::<FloorCostGrid>().clone();
    let surcharge = dismount_surcharge(world.get::<Mounted>(ganger), &tuning);
    let planning = PlanningView::new(&squad, all_other);
    reachable_within(
        departure,
        budget,
        surcharge,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
        &planning,
    )
}

#[test]
fn click_to_target_path_preview_still_works_with_overlay_off() {
    let links = VerticalLinkGraph::default();
    let mut app = reachable_app(links, false);

    let start = cell(10, 10, 0);
    let goal = cell(14, 12, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(start), BUDGET, PLAYER_FACTION))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.world_mut()
        .insert_resource(PathPreviewTarget::new(goal));

    advance_until(&mut app, |app| {
        !app.world().resource::<PathPreview>().is_empty()
    });

    let preview = app.world().resource::<PathPreview>();
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

    assert!(
        app.world().resource::<ReachableCells>().is_empty(),
        "the reachable overlay stays empty with the flag OFF even while the preview is shown",
    );
}
