//! `battle.reachable` over the real socket: what it lists, and what it refuses instead.

use cobalt_mcp_protocol::{
    command::{CommandOutcome, UnavailableCode},
    message::QaResponse,
};
use gdtf_game::{
    qa_wire::{cost::CostRefusalNet, token::GangerToken},
    test_support::ShownOccupancyGrid,
};

use super::support::{
    ReachableBody, assert_same_cells, first_reachable_body, listed_cells, reachable_call,
    reachable_on_screen,
};
use crate::{
    battle_cost::support::settle,
    battle_reads::{a_player_ganger, an_enemy_ganger, token_of},
    battle_setup::hold_the_screen_still,
    command_exchange::exchange_in_battle,
    socket_support::TestResult,
};

/// A token no ganger can hold: entity bits the world never hands out.
const NO_SUCH_TOKEN: u64 = u64::MAX;

#[test]
fn the_read_lists_every_cell_the_search_reached_with_what_reaching_it_charges() -> TestResult {
    let mut actor = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some((entity, _)) = a_player_ganger(app) else {
            return Vec::new();
        };
        if hold_the_screen_still(app).is_err() {
            return Vec::new();
        }
        actor = Some(entity);
        vec![reachable_call(token_of(entity))]
    })?;
    let Some(actor) = actor else {
        return Err(
            "a settled battle must field a player ganger with the screen held still".into(),
        );
    };
    let body = first_reachable_body(replies)?;
    let listed = listed_cells(&body)?;
    let Some(reached) = reachable_on_screen(&app, actor) else {
        return Err("the sim's own search must run over the picture the screen has drawn".into());
    };

    assert!(
        !reached.is_empty(),
        "a ganger with a TU pool standing on a generated map reaches somewhere, or this case \
         proves nothing: the search returned no cell at all",
    );
    assert!(
        reached.iter().any(|(_, cost)| **cost > 0),
        "the search must reach at least one cell that costs something, or a quoted zero would \
         pass: {reached:?}",
    );
    assert_same_cells(listed, &reached);
    Ok(())
}

#[test]
fn a_token_no_ganger_holds_is_refused_rather_than_answered_with_an_empty_set() -> TestResult {
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        vec![reachable_call(GangerToken::new(NO_SUCH_TOKEN))]
    })?;
    let body = first_reachable_body(replies)?;
    assert_refused(&body, CostRefusalNet::NoSuchGanger, NO_SUCH_TOKEN);
    Ok(())
}

#[test]
fn an_enemys_token_is_refused_rather_than_answered_with_an_empty_set() -> TestResult {
    let mut enemy = None;
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some(entity) = an_enemy_ganger(app) else {
            return Vec::new();
        };
        enemy = Some(entity);
        vec![reachable_call(token_of(entity))]
    })?;
    let Some(enemy) = enemy else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let body = first_reachable_body(replies)?;
    assert_refused(&body, CostRefusalNet::NotYourGanger, *token_of(enemy));
    Ok(())
}

#[test]
fn a_screen_with_no_drawn_grid_answers_unavailable_rather_than_an_empty_set() -> TestResult {
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some((entity, _)) = a_player_ganger(app) else {
            return Vec::new();
        };
        app.world_mut().remove_resource::<ShownOccupancyGrid>();
        vec![reachable_call(token_of(entity))]
    })?;
    let Some(reply) = replies.into_iter().next() else {
        return Err("a settled battle must field a player ganger to ask about".into());
    };
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        return Err(format!(
            "`battle.reachable` must refuse out of band when the screen has drawn no grid to \
             search over, got {reply:?}"
        )
        .into());
    };
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "the screen having drawn no grid is a missing model, not a wrong host state. note: {note:?}",
    );
    Ok(())
}

/// Require the reply to carry `refusal` and no cell list at all.
fn assert_refused(body: &ReachableBody, refusal: CostRefusalNet, token: u64) {
    assert_eq!(
        body.refusal,
        Some(refusal),
        "the token {token} must be refused {refusal:?} rather than answered; the reply carried \
         {body:?}",
    );
    assert!(
        body.cells.is_none(),
        "a refusal carries no cell list, so an empty set never reads as an answer to \
         \"which ganger?\": the token {token} came back as {body:?}",
    );
}
