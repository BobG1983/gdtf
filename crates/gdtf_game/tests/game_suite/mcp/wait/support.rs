use core::time::Duration;
use std::{
    cell::Cell,
    sync::mpsc::{Receiver, Sender},
};

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_host::{IncomingRequest, dispatch::DeferredBudget};
use cobalt_mcp_protocol::message::McpResponse;
use cobalt_test_utils::advance_until;
use gdtf_battle_presenter::playback::{ActHold, PlaybackCursor};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct},
    ganger::Faction,
};
use gdtf_game::test_support::shorten_wait_budget;

use crate::mcp::battle_fixture::{drive_into_battle_running, menu_app_with_mcp};

/// Longer than any case can drive, so only an explicit release opens the playback gate.
const UNENDING_HOLD_SECONDS: f32 = 3600.0;

/// Frames a parked call must survive before the case makes its condition true.
pub(crate) const PARKED_FRAMES: u32 = 8;

/// The menu fixture with `wait`'s own budget lifted, so only its condition releases the reply.
pub(crate) fn menu_app_with_unending_wait() -> (App, Sender<IncomingRequest>) {
    let (mut app, tx) = menu_app_with_mcp();
    shorten_wait_budget(&mut app, DeferredBudget::new(Duration::MAX));
    (app, tx)
}

/// A live battle whose playback cursor is holding, so the screen cannot be caught up.
pub(crate) fn holding_battle_with_mcp() -> (App, Sender<IncomingRequest>) {
    let (mut app, tx) = menu_app_with_unending_wait();
    drive_into_battle_running(&mut app);
    assert!(
        app.world().contains_resource::<ActLog>(),
        "a running battle must carry an act log for the playback gate to close on",
    );
    app.world_mut()
        .resource_mut::<PlaybackCursor>()
        .hold_for(ActHold::timed(UNENDING_HOLD_SECONDS));
    (app, tx)
}

/// Drop the hold and put the cursor on the log head, so the gate is open with no timing in it.
pub(crate) fn release_the_hold(app: &mut App) {
    let head = app.world().resource::<ActLog>().head();
    let mut cursor = app.world_mut().resource_mut::<PlaybackCursor>();
    cursor.reset();
    cursor.jump_to(head);
}

/// Step frames until the reply channel answers, with no cap on how long that takes.
pub(crate) fn answered_within(app: &mut App, reply: &Receiver<McpResponse>) -> McpResponse {
    let landed = Cell::new(None);
    loop {
        advance_until(app, |_| {
            let arrived = reply.try_recv().ok();
            let answered = arrived.is_some();
            landed.set(arrived);
            answered
        });
        if let Some(answer) = landed.take() {
            return answer;
        }
    }
}

/// Entries the live act log holds, or none at all before a battle.
pub(crate) fn act_log_len(app: &App) -> usize {
    app.world().get_resource::<ActLog>().map_or(0, ActLog::len)
}

/// Write `lines` turn-began entries into the live act log.
pub(crate) fn append_act_log_lines(app: &mut App, lines: usize) {
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("a running battle carries the sim's act log");
    };
    for _ in 0..lines {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ActProvenance::Clock,
            ActDeed::TurnBegan {
                now_active: Faction::new(0),
            },
            ActWitnesses::unseen(),
        ));
    }
}
