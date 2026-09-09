use bevy::{ecs::system::SystemState, prelude::*};
use gdtf_battle_input::{
    InspectTarget, LeftClickOutcome, PathPreviewTarget, SelectedShooter, decide_turn,
};
use gdtf_battle_sim::{
    acts::{MoveRequested, SetFacingRequested},
    prelude::{Cell, CellLevel, Direction, Level, Position},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
};

use super::harness::*;

fn link_graph_with_link(from: CellLevel, to: CellLevel) -> VerticalLinkGraph {
    let link = VerticalLink::new(from, to, LinkKind::stair());
    let situation = SituationBuilder::new()
        .slab_at(from)
        .slab_at(to)
        .vertical_link(link)
        .build();
    build_vertical_link_graph(&situation).unwrap_or_default()
}

fn turn(app: &mut App) -> Option<SetFacingRequested> {
    let world = app.world_mut();
    let mut state: SystemState<(Res<SelectedShooter>, Res<InspectTarget>, Query<&Position>)> =
        SystemState::new(world);
    let Ok((selected, hovered, positions)) = state.get(world) else {
        return None;
    };
    decide_turn(&selected, &hovered, &positions)
}

#[test]
fn decide_left_click_matches_the_contract_precedence() {
    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let target = CellLevel::new(Cell::new(6, 2), LEVEL);
        let _enemy = place_enemy(&mut app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
        seed_fog(&mut app, &[target], &[]);

        let outcome = decide(&mut app);
        assert!(
            matches!(outcome, LeftClickOutcome::Fire(_)),
            "an enemy cell with a fire mode must FIRE, got {outcome:?}",
        );
        let LeftClickOutcome::Fire(request) = outcome else {
            return;
        };
        assert_eq!(request.shooter, ganger, "FIRE shooter = the selection");
        assert_eq!(
            request.target_cell,
            Cell::new(6, 2),
            "FIRE target = hovered"
        );
    }

    {
        let mut app = decision_app();
        let cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, cell);
        app.world_mut().insert_resource(SelectedShooter::cleared());
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(cell)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Select(ganger),
            "a player-occupied cell must SELECT that ganger",
        );
    }

    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let dest = CellLevel::new(Cell::new(4, 3), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(dest)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::SetMoveTarget(dest),
            "click-1 over an empty cell with a selection must SET the move target",
        );
        app.world_mut()
            .insert_resource(PathPreviewTarget::new(dest));
        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Move(MoveRequested::new(ganger, dest)),
            "a click on the SAME cell as the current target must COMMIT the move",
        );
    }

    {
        let mut app = decision_app();
        app.world_mut().insert_resource(SelectedShooter::cleared());
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::NoOp,
            "nothing hovered must be a NO-OP, not CLEAR",
        );
    }

    {
        let mut app = decision_app();
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().insert_resource(SelectedShooter::new(stale));
        let cell = CellLevel::new(Cell::new(6, 6), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(cell)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Clear,
            "an in-grid Some cell with a non-player/stale selection (no FIRE/SELECT/MOVE/NO-OP) \
             must still CLEAR",
        );
    }
}

#[test]
fn fire_mode_over_empty_cell_falls_through_to_move() {
    let mut app = decision_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    let dest = CellLevel::new(Cell::new(11, 10), LEVEL);
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(dest)));

    assert_eq!(
        decide(&mut app),
        LeftClickOutcome::SetMoveTarget(dest),
        "a fire mode over an EMPTY cell must fall through to the move path (SetMoveTarget), \
         not FIRE / CLEAR",
    );
}

#[test]
fn decide_left_click_on_a_link_tile_is_a_no_op() {
    let mut app = decision_app();
    let shooter_cell = CellLevel::new(Cell::new(7, 7), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    let link_cell = CellLevel::new(Cell::new(8, 7), LEVEL);
    let up_cell = CellLevel::new(Cell::new(8, 7), Level::new(1));
    let graph = link_graph_with_link(link_cell, up_cell);
    app.world_mut().insert_resource(graph);
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(link_cell)));

    assert_eq!(
        decide(&mut app),
        LeftClickOutcome::NoOp,
        "a click on a vertical-link tile is a NO-OP (not a move target)",
    );
}

#[test]
fn decide_turn_matches_the_contract() {
    {
        let mut app = decision_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(CellLevel::new(
                Cell::new(8, 5),
                LEVEL,
            ))));

        let request = turn(&mut app);
        assert!(
            request.is_some(),
            "a hovered cell off the actor's cell must yield a turn",
        );
        let Some(request) = request else {
            return;
        };
        assert_eq!(request.actor, ganger, "the turn actor = the selection");
        assert_eq!(
            request.facing,
            Direction::East,
            "from_cells((5,5),(8,5)) must be East",
        );
    }

    {
        let mut app = decision_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(actor_cell)));

        assert_eq!(
            turn(&mut app),
            None,
            "hovering the actor's OWN cell must yield no turn",
        );
    }

    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, CellLevel::new(Cell::new(5, 5), LEVEL));
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(turn(&mut app), None, "nothing hovered must yield no turn");
    }
}
