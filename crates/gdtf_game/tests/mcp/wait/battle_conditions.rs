use std::sync::mpsc::TryRecvError;

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
use gdtf_battle_sim::{
    acts::{EndTurnRequested, movement::WalkInProgress},
    ganger::Tu,
    metric::{Cell, CellLevel, Level},
};
use gdtf_game::test_support::{BattleRunningComplete, GenerationComplete};

use super::support::{
    PARKED_FRAMES, SETTLE_FRAMES, act_log_len, answered_within, append_act_log_lines,
};
use crate::{
    battle_fixture::{drive_into_battle_running, menu_app_with_mcp, run_request, send},
    command_exchange::WAIT,
};

/// Entries the deleted ring buffer capped the act log at.
const DELETED_CAP: usize = 2048;

/// Put one route in flight, carrying the component `dispatch_move` inserts on a real walk.
fn start_a_walk(app: &mut App) -> Entity {
    let route = [CellLevel::new(Cell::new(1, 1), Level::new(0))];
    let costs = [Tu::new(1)];
    app.world_mut()
        .spawn(WalkInProgress::new(&route, &costs))
        .id()
}

#[test]
fn wait_on_generation_complete_parks_in_the_menu_and_settles_while_the_situation_generates() {
    let (mut app, tx) = menu_app_with_mcp();

    let reply = send(&tx, run_request(WAIT, "(condition:GenerationComplete)"));
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert!(
            !app.world().contains_resource::<GenerationComplete>(),
            "nothing generates in the Menu, so the marker must still be absent on frame {frame}",
        );
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "no situation has been generated, so the wait must stay parked — it answered on \
             frame {frame}",
        );
    }

    drive_into_battle_running(&mut app);

    let answered = reply.try_recv();
    assert!(
        matches!(
            answered,
            Ok(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "generating a situation is what this condition waits for, so the parked wait must have \
         been released while the marker was up; got {answered:?}",
    );
}

#[test]
fn wait_on_turn_changed_parks_until_a_turn_actually_hands_over() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);

    let reply = send(&tx, run_request(WAIT, "(condition:TurnChanged)"));
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "no turn has handed over since the call was admitted, so the wait must stay parked — \
             it answered on frame {frame}",
        );
    }

    app.world_mut().write_message(EndTurnRequested);

    let answered = answered_within(&mut app, &reply, SETTLE_FRAMES);
    assert!(
        matches!(
            answered,
            Some(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "ending the turn is what emits the TurnStarted this condition counts, so the parked wait \
         must be released; got {answered:?}",
    );
}

#[test]
fn wait_on_the_act_log_answers_only_once_the_log_holds_the_entries_asked_for() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);

    let wanted = act_log_len(&app) + 1;
    let reply = send(
        &tx,
        run_request(WAIT, &format!("(condition:LogAtLeast({wanted}))")),
    );
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert!(
            act_log_len(&app) < wanted,
            "nobody acts while the case holds the battle still, so the log must not reach \
             {wanted} entries on its own — it did on frame {frame}",
        );
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "the log is an entry short of {wanted}, so the wait must stay parked — it answered \
             on frame {frame}",
        );
    }

    app.world_mut().write_message(EndTurnRequested);

    let answered = answered_within(&mut app, &reply, SETTLE_FRAMES);
    assert!(
        matches!(
            answered,
            Some(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "a turn hand-off is recorded in the act log, which takes it to {wanted} entries; got \
         {answered:?}",
    );
    assert!(
        act_log_len(&app) >= wanted,
        "and the release must be the log actually growing, not the condition going soft",
    );
}

#[test]
fn wait_on_the_act_log_answers_for_a_count_past_the_deleted_ring_buffer_cap() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);

    let wanted = DELETED_CAP.saturating_add(1);
    append_act_log_lines(&mut app, wanted);
    assert!(
        act_log_len(&app) >= wanted,
        "the log must actually hold more than {DELETED_CAP} entries, or this case proves \
         nothing: {wanted} lines were written and it holds {held}",
        held = act_log_len(&app),
    );

    let reply = send(
        &tx,
        run_request(WAIT, &format!("(condition:LogAtLeast({wanted}))")),
    );

    let answered = answered_within(&mut app, &reply, SETTLE_FRAMES);
    assert!(
        matches!(
            answered,
            Some(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "the log already holds {wanted} entries, so a wait asking for that many must answer \
         rather than park: the log holds {held}; got {answered:?}",
        held = act_log_len(&app),
    );
}

#[test]
fn wait_on_walk_complete_parks_while_someone_is_part_way_through_a_walk() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let walker = start_a_walk(&mut app);

    let reply = send(&tx, run_request(WAIT, "(condition:WalkComplete)"));
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert!(
            app.world().get::<WalkInProgress>(walker).is_some(),
            "the route must still be in flight on frame {frame} or this case proves nothing",
        );
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "a walk is still running, so the wait must stay parked — it answered on frame \
             {frame}",
        );
    }

    app.world_mut()
        .entity_mut(walker)
        .remove::<WalkInProgress>();

    let answered = answered_within(&mut app, &reply, SETTLE_FRAMES);
    assert!(
        matches!(
            answered,
            Some(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "with the last WalkInProgress gone nobody is mid-walk, so the parked wait must be \
         released; got {answered:?}",
    );
}

#[test]
fn wait_on_battle_decided_settles_on_the_same_marker_the_flee_button_inserts() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);

    let reply = send(&tx, run_request(WAIT, "(condition:BattleDecided)"));
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "the battle is still being fought, so nothing has decided it — the wait answered on \
             frame {frame}",
        );
    }

    app.world_mut().insert_resource(BattleRunningComplete);

    let answered = answered_within(&mut app, &reply, SETTLE_FRAMES);
    assert!(
        matches!(
            answered,
            Some(McpResponse::Outcome(CommandOutcome::Ran { .. }))
        ),
        "the marker the Flee button and the outcome watcher both insert is what decides a \
         battle, so the parked wait must be released; got {answered:?}",
    );
}
