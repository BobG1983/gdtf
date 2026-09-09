//! `battle.reachable` plans over the grid, fog and start cell the screen draws.

use gdtf_battle_sim::prelude::OccupancyGrid;

use crate::mcp::{
    battle_cost::support::settle,
    battle_reachable::support::{
        assert_same_cells, drawn_start_and_budget, first_reachable_body, listed_cells, lists,
        reachable_call, reachable_from, reachable_on_screen,
    },
    battle_reads::{a_player_ganger, an_enemy_ganger, token_of},
    battle_setup::{
        battle_with_a_player_ganger_the_screen_has_not_moved, forget_on_screen_only,
        hold_the_screen_still, shown_fog_lights,
    },
    command_exchange::{exchange_in_battle, exchange_inspecting},
    socket_support::TestResult,
};

#[test]
fn the_reachable_read_walks_the_grid_the_screen_draws_not_the_one_the_sim_has_written() -> TestResult
{
    let mut occupied = None;
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        let (Some((actor, _)), Some(blocker)) = (a_player_ganger(app), an_enemy_ganger(app)) else {
            return Vec::new();
        };
        if hold_the_screen_still(app).is_err() {
            return Vec::new();
        }
        let (Some((start, _)), Some(reached)) = (
            drawn_start_and_budget(app, actor),
            reachable_on_screen(app, actor),
        ) else {
            return Vec::new();
        };
        // A cell the fog leaves dark blocks on neither grid, so the live write would not bite.
        let Some(at) = reached
            .iter()
            .map(|(at, _)| *at)
            .find(|at| *at != start && *shown_fog_lights(app, *at))
        else {
            return Vec::new();
        };
        let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
            return Vec::new();
        };
        grid.set_occupant(at, Some(blocker));
        occupied = Some(at);
        vec![reachable_call(token_of(actor))]
    })?;
    let Some(at) = occupied else {
        return Err(
            "the map must offer a player ganger a cell it reaches on the screen's own \
                    grid that the screen's own fog still lights"
                .into(),
        );
    };
    let body = first_reachable_body(replies)?;
    let listed = listed_cells(&body)?;

    assert!(
        lists(listed, at),
        "the screen still draws {at:?} as open, so the read still reaches it; only the sim's \
         own grid has an enemy standing there: {listed:?}",
    );
    Ok(())
}

#[test]
fn the_reachable_read_plans_through_the_fog_the_screen_draws_not_the_sims_own() -> TestResult {
    let mut forgotten = None;
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some((actor, _)) = a_player_ganger(app) else {
            return Vec::new();
        };
        if hold_the_screen_still(app).is_err() {
            return Vec::new();
        }
        let (Some((start, _)), Some(reached)) = (
            drawn_start_and_budget(app, actor),
            reachable_on_screen(app, actor),
        ) else {
            return Vec::new();
        };
        let Some(at) = reached.iter().map(|(at, _)| *at).find(|at| *at != start) else {
            return Vec::new();
        };
        if forget_on_screen_only(app, at).is_err() {
            return Vec::new();
        }
        forgotten = Some(at);
        vec![reachable_call(token_of(actor))]
    })?;
    let Some(at) = forgotten else {
        return Err(
            "the map must offer a player ganger a cell it reaches other than the one it stands on"
                .into(),
        );
    };
    let body = first_reachable_body(replies)?;
    let listed = listed_cells(&body)?;

    assert!(
        !lists(listed, at),
        "the screen's fog no longer remembers {at:?}, so the read plans no route through it, \
         however well the sim's own fog still knows it: {listed:?}",
    );
    Ok(())
}

#[test]
fn the_reachable_read_plans_from_the_cell_the_screen_draws_not_the_one_the_sim_moved_it_to()
-> TestResult {
    let (app, replies, mover) = exchange_inspecting(
        battle_with_a_player_ganger_the_screen_has_not_moved,
        |mover| vec![reachable_call(token_of(mover.entity))],
    )?;
    let body = first_reachable_body(replies)?;
    let listed = listed_cells(&body)?;
    let Some((start, budget)) = drawn_start_and_budget(&app, mover.entity) else {
        return Err("the ganger the fixture split must still be standing somewhere".into());
    };
    assert_eq!(
        start, mover.drawn,
        "the fixture must draw the ganger on the cell it split off: {mover:?}",
    );
    let (Some(from_drawn), Some(from_live)) = (
        reachable_from(&app, mover.entity, mover.drawn, budget),
        reachable_from(&app, mover.entity, mover.live, budget),
    ) else {
        return Err("the sim's own search must run from either cell over the screen's grid".into());
    };
    let drawn_cells: Vec<_> = from_drawn.iter().map(|(at, _)| *at).collect();
    let live_cells: Vec<_> = from_live.iter().map(|(at, _)| *at).collect();

    assert_ne!(
        drawn_cells, live_cells,
        "the fixture must really tell the drawn cell from the live one, or this case proves \
         nothing: {mover:?}",
    );
    assert_same_cells(listed, &from_drawn);
    Ok(())
}
