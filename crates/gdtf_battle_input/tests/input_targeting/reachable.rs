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
    battle::PlayerFaction,
    floor::FloorCostGrid,
    metric::MAX_LEVELS,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    pathfinder::{MoveGrids, PlanningView, reachable_within},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    test_support::{SituationBuilder, key},
    tuning::CombatTuning,
    vertical::{VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
    visibility::{FactionRelation, SquadVisibility},
};
use gdtf_test_utils::advance_until;

const PLAYER_FACTION: Faction = Faction::new(0);

const BUDGET: Tu = Tu::new(200);

fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
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
            foot,
            BUDGET,
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
