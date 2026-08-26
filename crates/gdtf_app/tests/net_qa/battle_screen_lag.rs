//! The battle reads answer with the picture on screen, not with the sim running ahead of it.

use gdtf_app::qa_wire::{
    inspect::InspectShownNet, roster::GangerCardNet, token::GangerToken, visible::VisibleGangerNet,
};
use gdtf_battle_sim::prelude::OccupancyGrid;
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use serde::Deserialize;

use super::{
    battle_cost::support::settle,
    battle_reachable::support::{
        assert_same_cells, drawn_start_and_budget, first_reachable_body, listed_cells, lists,
        reachable_call, reachable_from, reachable_on_screen,
    },
    battle_reads::{a_player_ganger, an_enemy_ganger, cell_argument, token_of},
    battle_setup::{
        Standing, battle_with_a_frozen_fog, battle_with_a_ganger_the_screen_has_not_moved,
        battle_with_a_player_ganger_the_screen_has_not_moved,
        battle_with_an_occupant_the_screen_has_not_seen, forget_on_screen_only,
        hold_the_screen_still, shown_fog_lights,
    },
    command_exchange::{
        BATTLE_INSPECT, BATTLE_ROSTER, BATTLE_VISIBLE, exchange_expected, exchange_in_battle,
        exchange_inspecting, ran_body, run,
    },
    socket_support::{TestError, TestResult},
};

#[derive(Debug, Deserialize)]
struct VisibleBody {
    enemies: Vec<VisibleGangerNet>,
}

#[derive(Debug, Deserialize)]
struct RosterBody {
    gangers: Vec<GangerCardNet>,
}

#[derive(Debug, Deserialize)]
struct InspectBody {
    shown: InspectShownNet,
}

fn decode<T: serde::de::DeserializeOwned>(
    name: &'static str,
    reply: Option<QaResponse>,
) -> Result<T, TestError> {
    let Some(reply) = reply else {
        return Err(format!("`{name}` produced no reply").into());
    };
    let body = ran_body(name, reply)?;
    ron::de::from_str::<T>(&body)
        .map_err(|fault| format!("`{name}`'s body must decode: {fault} — {body}").into())
}

fn both_reads() -> Vec<QaRequest> {
    vec![
        run(BATTLE_VISIBLE, "()", RunOptions::default()),
        run(BATTLE_ROSTER, "()", RunOptions::default()),
    ]
}

fn split(replies: Vec<QaResponse>) -> Result<(VisibleBody, RosterBody), TestError> {
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, replies.next())?;
    Ok((visible, roster))
}

#[test]
fn an_enemy_is_read_where_the_screen_draws_it_not_where_the_sim_moved_it() -> TestResult {
    let (replies, enemy) = exchange_expected(
        || battle_with_a_ganger_the_screen_has_not_moved(Standing::Lit),
        |_enemy| both_reads(),
    )?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(entry) = visible.enemies.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "the sprite still stands in the lit area, so the read lists it: {enemy:?} missing \
             from {visible:?}"
        );
    };
    assert_eq!(
        entry.at, enemy.drawn,
        "the entry names the cell the sprite is drawn on, not the one the sim moved it to: \
         {entry:?} against {enemy:?}",
    );
    assert_ne!(
        enemy.drawn, enemy.live,
        "the fixture must really split the two cells, or this case proves nothing",
    );
    let Some(card) = roster.gangers.iter().find(|card| card.token == token) else {
        unreachable!(
            "the roster judges from the same drawn cell, so it lists what the screen shows too: \
             {enemy:?} — {roster:?}"
        );
    };
    assert_eq!(
        card.at, enemy.drawn,
        "the card places the ganger on the cell its sprite is drawn on, the same cell the lit \
         area named: {card:?} against {enemy:?}",
    );
    Ok(())
}

#[test]
fn an_enemy_the_screen_still_hides_stays_out_of_both_reads() -> TestResult {
    let (replies, enemy) = exchange_expected(
        || battle_with_a_ganger_the_screen_has_not_moved(Standing::Hidden),
        |_enemy| both_reads(),
    )?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    assert!(
        !visible.enemies.iter().any(|entry| entry.token == token),
        "the sim has walked it into the light but the sprite is still in the dark, so the lit \
         area leaves it out: {enemy:?} appears in {visible:?}",
    );
    assert!(
        !roster.gangers.iter().any(|card| card.token == token),
        "the roster leaves it out for the same reason: {enemy:?} appears in {roster:?}",
    );
    Ok(())
}

#[test]
fn a_cell_the_screens_fog_still_lights_reads_as_lit() -> TestResult {
    let (replies, enemy) = exchange_expected(battle_with_a_frozen_fog, |_enemy| both_reads())?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(entry) = visible.enemies.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "the screen's fog still lights that cell, so the enemy standing on it is in the \
             lit area: {enemy:?} missing from {visible:?}"
        );
    };
    assert_eq!(
        entry.at, enemy.at,
        "the entry names the cell it stands on: {entry:?}",
    );
    assert!(
        roster.gangers.iter().any(|card| card.token == token),
        "the roster reads the same frozen fog, so the enemy still has a card: {enemy:?} — \
         {roster:?}",
    );
    Ok(())
}

#[test]
fn the_inspect_read_ignores_an_occupant_the_screen_has_not_shown_arriving() -> TestResult {
    let (replies, occupant) = exchange_expected(
        battle_with_an_occupant_the_screen_has_not_seen,
        |occupant| {
            vec![run(
                BATTLE_INSPECT,
                &cell_argument(occupant.at),
                RunOptions::default(),
            )]
        },
    )?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.into_iter().next())?;

    assert_eq!(
        inspect.shown.ganger, None,
        "the panel draws no card on a cell whose occupant the screen has not shown arriving: \
         {occupant:?} showed as {inspect:?}",
    );
    Ok(())
}

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
