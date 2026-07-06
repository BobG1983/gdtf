//! The shared `decide_left_click` / `decide_turn` contract precedence (AC2).

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

// =================================================================================
// AC2 — the SHARED `decide_left_click` / `decide_turn` match the contract precedence.
// =================================================================================

/// A [`VerticalLinkGraph`] holding ONE stair link `from → to` (built through the real
/// [`build_vertical_link_graph`] validation off a minimal [`Situation`] with slabs at both
/// endpoints, so `links_from(from)` reports `from` as a vertical-link tile — the GTW-356 OQ-4
/// non-target gate). The `Err` arm is structurally impossible (both endpoints are authored
/// slabs and on distinct storeys), so it falls back to an empty graph rather than panicking
/// (the no-`unwrap` test rule).
fn link_graph_with_link(from: CellLevel, to: CellLevel) -> VerticalLinkGraph {
    let link = VerticalLink::new(from, to, LinkKind::stair());
    let situation = SituationBuilder::new()
        .slab_at(from)
        .slab_at(to)
        .vertical_link(link)
        .build();
    build_vertical_link_graph(&situation).unwrap_or_default()
}

/// Calls the SHARED [`decide_turn`] over the `app`'s world via a `SystemState` constructed in
/// this helper's body (carve-out (a) — `app.world_mut()`, NOT a `&mut World` signature).
fn turn(app: &mut App) -> Option<SetFacingRequested> {
    let world = app.world_mut();
    let mut state: SystemState<(Res<SelectedShooter>, Res<InspectTarget>, Query<&Position>)> =
        SystemState::new(world);
    // `get` now returns a `Result` (Bevy 0.19); these params always validate.
    let Ok((selected, hovered, positions)) = state.get(world) else {
        return None;
    };
    decide_turn(&selected, &hovered, &positions)
}

/// AC2 — `decide_left_click` resolves the FIRE → SELECT → MOVE → CLEAR precedence into the
/// matching `LeftClickOutcome` (the same decision the mouse AND the gamepad use). Each branch
/// is pin-discriminated against a WRONG variant.
#[test]
fn decide_left_click_matches_the_contract_precedence() {
    // --- FIRE: a fire mode + a player selection + an ENEMY occupant + can_fire passes. ---
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
        seed_fog(&mut app, &[target], &[]); // GTW-11: target must be VISIBLE for the fire commit.

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

    // --- SELECT: the hovered cell holds one of YOUR gangers. ---
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

    // --- MOVE (two-click, GTW-356): click-1 (no target) SETS it; a click on the SAME cell COMMITS.
    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let dest = CellLevel::new(Cell::new(4, 3), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(dest)));

        // Click-1 (default target None) -> SET the target, not a commit.
        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::SetMoveTarget(dest),
            "click-1 over an empty cell with a selection must SET the move target (GTW-356)",
        );
        // With the target now equal to the cell, a click on the SAME cell COMMITS.
        app.world_mut()
            .insert_resource(PathPreviewTarget::new(dest));
        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Move(MoveRequested::new(ganger, dest)),
            "a click on the SAME cell as the current target must COMMIT the move (GTW-356)",
        );
    }

    // --- NO-OP: nothing hovered (GTW-288). The GTW-286 viewport gate resolves an over-UI /
    //     margin / off-map click to no hovered cell, and a no-hover click must NOT clear the
    //     selection — it is a NoOp (the gamepad shares the same `decide_left_click`). ---
    {
        let mut app = decision_app();
        app.world_mut().insert_resource(SelectedShooter::cleared());
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::NoOp,
            "nothing hovered must be a NO-OP, not CLEAR (GTW-288, the GTW-286 over-UI case)",
        );
    }

    // --- CLEAR: a valid in-grid hovered cell with a NON-player / stale selection where none
    //     of FIRE/SELECT/MOVE/NO-OP applies (GTW-288 keeps CLEAR for the genuine Some-cell
    //     case). A bare entity selection is not player-faction, so MOVE does not apply. ---
    {
        let mut app = decision_app();
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().insert_resource(SelectedShooter::new(stale));
        // An EMPTY in-grid cell with a non-player (bare-entity) selection: FIRE needs an enemy
        // occupant (none), SELECT needs a player occupant (none), MOVE needs a player-faction
        // selection (the bare `stale` is not), NO-OP needs an enemy occupant (none) -> CLEAR.
        let cell = CellLevel::new(Cell::new(6, 6), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(cell)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Clear,
            "an in-grid Some cell with a non-player/stale selection (no FIRE/SELECT/MOVE/NO-OP) \
             must still CLEAR (GTW-288 preserves the genuine clear case)",
        );
    }
}

/// AC2 fall-through — a fire mode over an EMPTY cell must NOT lock out the two-click MOVE path
/// (not FIRE, not CLEAR): with no current target it falls through to `SetMoveTarget`. Split out of
/// [`decide_left_click_matches_the_contract_precedence`] to keep each test under `too_many_lines`.
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

/// GTW-356 OQ-4 — the SHARED `decide_left_click` resolves a click on a VERTICAL-LINK tile to
/// `NoOp`: not a move target, not a move dispatch (the same decision the mouse AND the gamepad
/// use). Split out of [`decide_left_click_matches_the_contract_precedence`] to keep each test
/// under the `too_many_lines` lint.
#[test]
fn decide_left_click_on_a_link_tile_is_a_no_op() {
    let mut app = decision_app();
    let shooter_cell = CellLevel::new(Cell::new(7, 7), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    // A vertical-link tile at the clicked cell (a stair link up from it).
    let link_cell = CellLevel::new(Cell::new(8, 7), LEVEL);
    let up_cell = CellLevel::new(Cell::new(8, 7), Level::new(1));
    let graph = link_graph_with_link(link_cell, up_cell);
    app.world_mut().insert_resource(graph);
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(link_cell)));

    assert_eq!(
        decide(&mut app),
        LeftClickOutcome::NoOp,
        "a click on a vertical-link tile is a NO-OP (OQ-4: not a move target)",
    );
}

/// AC2 — `decide_turn` resolves the actor→hovered direction into a `SetFacingRequested`, and
/// returns `None` for the actor's OWN cell / no hover (the same decision the mouse AND the
/// gamepad use).
#[test]
fn decide_turn_matches_the_contract() {
    // (5,5) -> (8,5) is due East.
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

    // Hovering the actor's OWN cell -> from_cells None -> no turn.
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

    // No hover -> no turn.
    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, CellLevel::new(Cell::new(5, 5), LEVEL));
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(turn(&mut app), None, "nothing hovered must yield no turn");
    }
}
